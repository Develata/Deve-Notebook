//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Owner-specific post-cut cleanup and exact terminal settlement.

use super::LiveRemovalReservation;
use crate::server::runtime::repo_lifecycle_job_runtime::{
    RemovalCleanupDisposition, RemovalCleanupStep, RepoLifecycleJobCompletion,
    RepoLifecycleSettledPublication, RepoRemovalExecution, RepoRemovalFallbackSnapshot,
};
use crate::server::runtime::repo_lifecycle_runtime::RepoLifecycleCoordinator;
use deve_core::ledger::{
    HostRepoAliasCleanupDisposition, ProjectionLocatorCleanupDisposition,
    RepoAuthorityCleanupGuard, RepoCatalogMembershipState, RepoCatalogRetirementDisposition,
};
#[cfg(test)]
use std::sync::atomic::Ordering;

impl RepoLifecycleCoordinator {
    #[cfg(test)]
    pub(crate) fn fail_next_owned_cleanup_for_test(&self, step: RemovalCleanupStep) {
        self.fail_next_owned_cleanup_step
            .store(cleanup_step_code(step), Ordering::Release);
    }

    pub(super) async fn settle_owned_removal(
        &self,
        removal: &mut RepoRemovalExecution,
        authority: &mut RepoAuthorityCleanupGuard,
        mut live: Option<LiveRemovalReservation>,
    ) -> Result<RepoLifecycleJobCompletion, String> {
        let repo_id = removal.manifest.repo_id;
        for step in RemovalCleanupStep::ORDER {
            if removal.state.completed(step) {
                self.verify_completed_cleanup_step(removal, authority, step)?;
                continue;
            }
            #[cfg(test)]
            if self
                .fail_next_owned_cleanup_step
                .compare_exchange(
                    cleanup_step_code(step),
                    0,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return self
                    .fail_cleanup(removal, step, "injected owner cleanup failure".to_string())
                    .await;
            }
            let disposition = match step {
                RemovalCleanupStep::RemoteImportArtifacts => {
                    let plan = removal.state.remote_import_plan.clone().ok_or_else(|| {
                        "committed removal lacks sealed Remote Import plan".to_string()
                    })?;
                    let mut checkpoint = removal
                        .state
                        .remote_import_checkpoint
                        .clone()
                        .unwrap_or_else(|| plan.initial_checkpoint());
                    while !checkpoint.is_complete() {
                        checkpoint = match self
                            .remote_import
                            .advance_repo_removal(&plan, &checkpoint)
                        {
                            Ok(checkpoint) => checkpoint,
                            Err(error) => {
                                return self.fail_cleanup(removal, step, error.to_string()).await;
                            }
                        };
                        removal.state = removal
                            .progress
                            .remote_import_checkpoint(checkpoint.clone())
                            .await
                            .map_err(|error| error.to_string())?;
                    }
                    if let Err(error) = self
                        .remote_import
                        .verify_repo_removal_complete(&plan, &checkpoint)
                    {
                        return self.fail_cleanup(removal, step, error.to_string()).await;
                    }
                    RemovalCleanupDisposition::Deleted
                }
                RemovalCleanupStep::ProcessRuntimeSlots => {
                    let mut errors = Vec::new();
                    if let Some(live) = live.take() {
                        if let Err(error) = self.finish_provider(live.provider).await {
                            errors.push(format!("provider runtime: {error}"));
                        }
                        if let Err(error) = self.watchers.finalize_removed(live.watcher) {
                            errors.push(format!("watcher runtime: {error}"));
                        }
                    } else {
                        if let Err(error) =
                            self.remote_import.cleanup_removed_provider_runtime(repo_id)
                        {
                            errors.push(format!("provider runtime: {error}"));
                        }
                        if let Err(error) = self.watchers.cleanup_removed_repo_runtime(repo_id) {
                            errors.push(format!("watcher runtime: {error}"));
                        }
                    }
                    if !errors.is_empty() {
                        return self.fail_cleanup(removal, step, errors.join("; ")).await;
                    }
                    RemovalCleanupDisposition::Retired
                }
                RemovalCleanupStep::NotegitTree => {
                    let mut checkpoint = match removal.state.notegit_checkpoint.clone() {
                        Some(checkpoint) => checkpoint,
                        None => {
                            let checkpoint = removal.manifest.notegit.initial_checkpoint();
                            removal.state = removal
                                .progress
                                .notegit_checkpoint(checkpoint.clone())
                                .await
                                .map_err(|error| error.to_string())?;
                            checkpoint
                        }
                    };
                    while !checkpoint.is_complete() {
                        checkpoint = match removal.manifest.notegit.advance_cleanup(&checkpoint) {
                            Ok(checkpoint) => checkpoint,
                            Err(error) => {
                                return self.fail_cleanup(removal, step, error.to_string()).await;
                            }
                        };
                        removal.state = removal
                            .progress
                            .notegit_checkpoint(checkpoint.clone())
                            .await
                            .map_err(|error| error.to_string())?;
                    }
                    if let Err(error) = removal.manifest.notegit.verify_complete(&checkpoint) {
                        return self.fail_cleanup(removal, step, error.to_string()).await;
                    }
                    RemovalCleanupDisposition::Deleted
                }
                RemovalCleanupStep::LocalAuthorityDatabase => {
                    let mut checkpoint = match removal.state.authority_checkpoint.clone() {
                        Some(checkpoint) => checkpoint,
                        None => {
                            let checkpoint =
                                removal.manifest.authority.initial_database_checkpoint();
                            removal.state = removal
                                .progress
                                .authority_checkpoint(checkpoint.clone())
                                .await
                                .map_err(|error| error.to_string())?;
                            checkpoint
                        }
                    };
                    while !checkpoint.is_complete() {
                        checkpoint = match authority
                            .advance_database_cleanup(&removal.manifest.authority, &checkpoint)
                        {
                            Ok(checkpoint) => checkpoint,
                            Err(error) => {
                                return self.fail_cleanup(removal, step, error.to_string()).await;
                            }
                        };
                        removal.state = removal
                            .progress
                            .authority_checkpoint(checkpoint.clone())
                            .await
                            .map_err(|error| error.to_string())?;
                    }
                    if let Err(error) = authority
                        .verify_database_cleanup_complete(&removal.manifest.authority, &checkpoint)
                    {
                        return self.fail_cleanup(removal, step, error.to_string()).await;
                    }
                    RemovalCleanupDisposition::Deleted
                }
                RemovalCleanupStep::ProjectionLocator => {
                    match self
                        .repo
                        .cleanup_projection_locator_removal(&removal.manifest.locator)
                    {
                        Ok(ProjectionLocatorCleanupDisposition::Deleted) => {
                            RemovalCleanupDisposition::Deleted
                        }
                        Ok(ProjectionLocatorCleanupDisposition::AlreadyAbsent) => {
                            RemovalCleanupDisposition::AlreadyAbsent
                        }
                        Err(error) => {
                            return self.fail_cleanup(removal, step, error.to_string()).await;
                        }
                    }
                }
                RemovalCleanupStep::HostAlias => {
                    match self
                        .repo
                        .host_repo_alias_runtime()
                        .cleanup_removal(&removal.manifest.alias)
                    {
                        Ok(HostRepoAliasCleanupDisposition::Deleted) => {
                            RemovalCleanupDisposition::Deleted
                        }
                        Ok(HostRepoAliasCleanupDisposition::AlreadyAbsent) => {
                            RemovalCleanupDisposition::AlreadyAbsent
                        }
                        Err(error) => {
                            return self.fail_cleanup(removal, step, error.to_string()).await;
                        }
                    }
                }
            };
            removal.state = removal
                .progress
                .cleanup_step(step, disposition)
                .await
                .map_err(|error| error.to_string())?;
        }
        removal.state = removal
            .progress
            .cleanup_complete()
            .await
            .map_err(|error| error.to_string())?;
        if !removal.state.tombstone_retired {
            let tombstone = removal
                .state
                .tombstone()
                .ok_or_else(|| "cleanup complete has no exact catalog tombstone".to_string())?;
            let retired = self
                .gate
                .execute_catalog_repo_cut(repo_id, |permit| {
                    self.repo.retire_repo_removal_tombstone(tombstone, permit)
                })
                .await
                .map_err(|error| error.to_string())?
                .map_err(|error| error.to_string())?;
            debug_assert!(matches!(
                retired,
                RepoCatalogRetirementDisposition::Retired
                    | RepoCatalogRetirementDisposition::AlreadyAbsent
            ));
            removal.state = removal
                .progress
                .tombstone_retired()
                .await
                .map_err(|error| error.to_string())?;
        }
        let fallback_repo_id = removal
            .manifest
            .fallback
            .as_ref()
            .filter(|fallback| self.fallback_is_exact(fallback))
            .map(|fallback| fallback.repo_id);
        Ok(RepoLifecycleJobCompletion::succeeded(
            RepoLifecycleSettledPublication::Removed {
                repo_id,
                fallback_repo_id,
            },
        ))
    }

    fn verify_completed_cleanup_step(
        &self,
        removal: &RepoRemovalExecution,
        authority: &RepoAuthorityCleanupGuard,
        step: RemovalCleanupStep,
    ) -> Result<(), String> {
        let repo_id = removal.manifest.repo_id;
        let exact = match step {
            RemovalCleanupStep::RemoteImportArtifacts => removal
                .state
                .remote_import_plan
                .as_ref()
                .zip(removal.state.remote_import_checkpoint.as_ref())
                .is_some_and(|(plan, checkpoint)| {
                    self.remote_import
                        .verify_repo_removal_complete(plan, checkpoint)
                        .is_ok()
                }),
            RemovalCleanupStep::ProcessRuntimeSlots => {
                self.remote_import
                    .removed_provider_runtime_is_absent(repo_id)
                    .unwrap_or(false)
                    && self
                        .watchers
                        .removed_repo_runtime_is_absent(repo_id)
                        .unwrap_or(false)
            }
            RemovalCleanupStep::NotegitTree => removal
                .state
                .notegit_checkpoint
                .as_ref()
                .is_some_and(|checkpoint| {
                    removal.manifest.notegit.verify_complete(checkpoint).is_ok()
                }),
            RemovalCleanupStep::LocalAuthorityDatabase => removal
                .state
                .authority_checkpoint
                .as_ref()
                .is_some_and(|checkpoint| {
                    authority
                        .verify_database_cleanup_complete(&removal.manifest.authority, checkpoint)
                        .is_ok()
                }),
            RemovalCleanupStep::ProjectionLocator => self
                .repo
                .projection_locator_removal_is_absent(&removal.manifest.locator)
                .unwrap_or(false),
            RemovalCleanupStep::HostAlias => self
                .repo
                .host_repo_alias_runtime()
                .removal_is_absent(&removal.manifest.alias)
                .unwrap_or(false),
        };
        if exact {
            Ok(())
        } else {
            Err(format!(
                "completed {step:?} cleanup receipt no longer matches exact owner truth"
            ))
        }
    }

    async fn fail_cleanup(
        &self,
        removal: &mut RepoRemovalExecution,
        step: RemovalCleanupStep,
        detail: String,
    ) -> Result<RepoLifecycleJobCompletion, String> {
        removal.state = removal
            .progress
            .cleanup_step(step, RemovalCleanupDisposition::Failed)
            .await
            .map_err(|error| format!("{detail}; failed to persist owner failure: {error}"))?;
        Ok(
            RepoLifecycleJobCompletion::repair_required(format!("{step:?} cleanup failed"))
                .with_cleanup(detail),
        )
    }

    fn fallback_is_exact(&self, expected: &RepoRemovalFallbackSnapshot) -> bool {
        self.repo
            .repo_catalog_membership_record(expected.repo_id)
            .ok()
            .flatten()
            .is_some_and(|record| {
                record.state() == RepoCatalogMembershipState::Normal
                    && record.membership_revision() == expected.membership_revision
            })
            && self
                .repo
                .snapshot_local_authority_for_removal(expected.repo_id)
                .is_ok_and(|snapshot| snapshot.generation() == expected.authority_generation)
            && self
                .watchers
                .mounted_generation(expected.repo_id)
                .is_ok_and(|generation| generation == expected.watcher_generation)
    }
}

#[cfg(test)]
const fn cleanup_step_code(step: RemovalCleanupStep) -> u8 {
    match step {
        RemovalCleanupStep::RemoteImportArtifacts => 1,
        RemovalCleanupStep::ProcessRuntimeSlots => 2,
        RemovalCleanupStep::NotegitTree => 3,
        RemovalCleanupStep::LocalAuthorityDatabase => 4,
        RemovalCleanupStep::ProjectionLocator => 5,
        RemovalCleanupStep::HostAlias => 6,
    }
}

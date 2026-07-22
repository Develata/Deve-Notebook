//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! Owner-driven local-repo removal. The coordinator orders the irreversible
//! catalog cut and delegates every destructive step to the runtime that owns
//! the corresponding state.

mod finalization;
mod recovery;
mod settlement;

use super::RepoLifecycleCoordinator;
use crate::remote_import_runtime::ProviderQuiesceToken;
use crate::server::runtime::repo_lifecycle_job_runtime::{
    RemovalCutState, RepoLifecycleJobCompletion, RepoLifecycleJobOutcome, RepoRemovalExecution,
};
use crate::server::runtime::watcher_runtime::WatcherMountReservation;
use deve_core::ledger::{RepoAuthorityCleanupGuard, RepoCatalogMembershipState};
#[cfg(test)]
use std::sync::atomic::Ordering;

pub(super) struct LiveRemovalReservation {
    pub(super) watcher: WatcherMountReservation,
    pub(super) provider: ProviderQuiesceToken,
}

impl RepoLifecycleCoordinator {
    pub(crate) async fn remove_owned(
        &self,
        mut removal: RepoRemovalExecution,
    ) -> RepoLifecycleJobCompletion {
        let repo_id = removal.manifest.repo_id;
        if removal.preparation_id.is_nil()
            || removal.manifest.catalog.repo_id() != repo_id
            || removal.manifest.authority.repo_id() != repo_id
            || removal.manifest.locator.repo_id() != repo_id
            || removal.manifest.notegit.repo_id() != repo_id
            || removal.manifest.alias.repo_id() != repo_id
        {
            return RepoLifecycleJobCompletion::repair_required(
                "removal manifest crosses repository ownership boundaries",
            );
        }

        if let Some(completion) = removal.state.terminal_candidate().cloned() {
            return self
                .finalize_owned_removal_candidate(&mut removal, completion)
                .await;
        }

        let (mut cleanup_guard, live) = if let Some(tombstone) = removal.state.tombstone().cloned()
        {
            if !tombstone
                .confirms_removed_manifest(removal.execute_request_id, &removal.manifest_digest)
            {
                return RepoLifecycleJobCompletion::repair_required(
                    "durable removal cut does not bind the admitted manifest",
                );
            }
            let guard = match self
                .repo
                .resume_local_authority_cleanup(&removal.manifest.authority)
            {
                Ok(guard) => guard,
                Err(error) => {
                    return RepoLifecycleJobCompletion::repair_required(format!(
                        "cannot reacquire exact local-authority cleanup: {error}"
                    ));
                }
            };
            (guard, None)
        } else {
            match removal.state.cut {
                RemovalCutState::Attempted => {
                    match self.recover_attempted_cut(&mut removal).await {
                        Ok(started) => started,
                        Err(completion) => return completion,
                    }
                }
                RemovalCutState::NotAttempted => match self.start_owned_removal(&mut removal).await
                {
                    Ok(started) => started,
                    Err(completion) => return completion,
                },
                RemovalCutState::Observed { .. } => {
                    return RepoLifecycleJobCompletion::repair_required(
                        "observed removal cut lost its exact tombstone",
                    );
                }
            }
        };

        let completion = match self
            .settle_owned_removal(&mut removal, &mut cleanup_guard, live)
            .await
        {
            Ok(completion) => completion,
            Err(detail) => RepoLifecycleJobCompletion::repair_required(detail),
        };

        if completion.publication.is_none() {
            return completion;
        }
        removal.state = match removal
            .progress
            .terminal_candidate(completion.clone())
            .await
        {
            Ok(state) => state,
            Err(error) => {
                return RepoLifecycleJobCompletion::repair_required(format!(
                    "cleanup converged but TerminalCandidate could not be persisted: {error}"
                ));
            }
        };
        #[cfg(test)]
        if self
            .fail_next_authority_retirement
            .swap(false, Ordering::AcqRel)
        {
            return RepoLifecycleJobCompletion::repair_required(
                "TerminalCandidate persisted but authority retirement failed: injected failure",
            );
        }
        let Some(authority_checkpoint) = removal.state.authority_checkpoint.as_ref() else {
            return RepoLifecycleJobCompletion::repair_required(
                "TerminalCandidate persisted without authority cleanup checkpoint",
            );
        };
        if let Err(error) = self.repo.retire_local_authority_after_removal(
            cleanup_guard,
            &removal.manifest.authority,
            authority_checkpoint,
        ) {
            return RepoLifecycleJobCompletion::repair_required(format!(
                "TerminalCandidate persisted but authority retirement failed: {error}"
            ));
        }
        #[cfg(test)]
        if let Err(error) = self.install_terminal_completion_failure_for_test() {
            return RepoLifecycleJobCompletion::repair_required(error);
        }
        if let Err(error) = removal.progress.terminal_complete().await {
            return RepoLifecycleJobCompletion::repair_required(format!(
                "authority retired but terminal removal receipt could not be persisted: {error}"
            ));
        }
        completion
    }

    async fn start_owned_removal(
        &self,
        removal: &mut RepoRemovalExecution,
    ) -> Result<
        (RepoAuthorityCleanupGuard, Option<LiveRemovalReservation>),
        RepoLifecycleJobCompletion,
    > {
        let repo_id = removal.manifest.repo_id;
        let current = self
            .repo
            .repo_catalog_membership_record(repo_id)
            .map_err(|error| {
                RepoLifecycleJobCompletion::repair_required(format!(
                    "cannot classify removal catalog truth: {error}"
                ))
            })?;

        if current.as_ref() != Some(&removal.manifest.catalog)
            || current
                .as_ref()
                .is_some_and(|record| record.state() != RepoCatalogMembershipState::Normal)
        {
            return Err(RepoLifecycleJobCompletion::not_committed(
                "removal catalog membership changed before the durable cut",
            ));
        }

        let prepared = self
            .repo
            .prepare_repo_removal_membership(
                repo_id,
                removal.execute_request_id,
                &removal.manifest_digest,
            )
            .map_err(|error| RepoLifecycleJobCompletion::not_committed(error.to_string()))?;
        let watcher = self
            .gate
            .execute_catalog_repo_unpublished(repo_id, || {
                let current = self.repo.repo_catalog_membership_record(repo_id)?;
                if current.as_ref() != Some(&removal.manifest.catalog) {
                    return Err(anyhow::anyhow!(
                        "catalog membership changed before watcher reserve"
                    ));
                }
                let watcher = self.watchers.reserve_existing(repo_id)?;
                if watcher.previous_generation() != Some(removal.manifest.watcher_generation) {
                    let _ = self.watchers.cancel_unstarted(watcher);
                    return Err(anyhow::anyhow!("watcher generation changed before removal"));
                }
                Ok(watcher)
            })
            .await
            .map_err(|error| RepoLifecycleJobCompletion::not_committed(error.to_string()))?
            .map_err(|error| RepoLifecycleJobCompletion::not_committed(error.to_string()))?;

        if !self.static_removal_owners_are_exact(removal) {
            let _ = self.watchers.cancel_unstarted(watcher);
            return Err(RepoLifecycleJobCompletion::not_committed(
                "removal owner identity changed before provider quiescence",
            ));
        }

        let provider = match self.quiesce_provider(repo_id).await {
            Ok(provider) => provider,
            Err(error) => {
                let _ = self.watchers.cancel_unstarted(watcher);
                return Err(RepoLifecycleJobCompletion::not_committed(error.to_string()));
            }
        };
        let remote_plan = if let Some(plan) = removal.state.remote_import_plan.clone() {
            match self.remote_import.revalidate_sealed_repo_removal(
                repo_id,
                &removal.manifest.remote_import,
                &plan,
            ) {
                Ok(true) => plan,
                Ok(false) => {
                    return Err(self
                        .rollback_owned_removal(
                            watcher,
                            Some(provider),
                            None,
                            Some(&plan),
                            "sealed Remote Import owner plan is no longer exact".to_string(),
                        )
                        .await);
                }
                Err(error) => {
                    return Err(self
                        .rollback_owned_removal(
                            watcher,
                            Some(provider),
                            None,
                            Some(&plan),
                            error.to_string(),
                        )
                        .await);
                }
            }
        } else {
            let plan = match self
                .remote_import
                .seal_repo_removal(repo_id, &removal.manifest.remote_import)
            {
                Ok(plan) => plan,
                Err(error) => {
                    return Err(self
                        .rollback_owned_removal(
                            watcher,
                            Some(provider),
                            None,
                            None,
                            error.to_string(),
                        )
                        .await);
                }
            };
            removal.state = match removal.progress.seal_remote_import(plan.clone()).await {
                Ok(state) => state,
                Err(error) => {
                    return Err(self
                        .rollback_owned_removal(
                            watcher,
                            Some(provider),
                            None,
                            Some(&plan),
                            error.to_string(),
                        )
                        .await);
                }
            };
            plan
        };

        let watchers = self.watchers.clone();
        let stopped = tokio::task::spawn_blocking(move || {
            let result = super::mount::stop_reserved(&watchers, &watcher);
            (watcher, result)
        })
        .await;
        let (watcher, stopped) = match stopped {
            Ok(result) => result,
            Err(_) => {
                return Err(RepoLifecycleJobCompletion::repair_required(
                    "watcher stop task failed while reservation ownership was in flight",
                ));
            }
        };
        if let Err(error) = stopped {
            return Err(self
                .rollback_owned_removal(
                    watcher,
                    Some(provider),
                    None,
                    Some(&remote_plan),
                    error.to_string(),
                )
                .await);
        }

        let notegit_exact = removal.manifest.notegit.revalidate().unwrap_or(false);
        let authority_exact = removal.manifest.authority.revalidate().unwrap_or(false);
        let locator_exact = self
            .repo
            .prepare_projection_locator_removal(repo_id)
            .is_ok_and(|plan| plan == removal.manifest.locator);
        let alias_exact = self
            .repo
            .host_repo_alias_runtime()
            .prepare_removal(repo_id)
            .is_ok_and(|plan| plan == removal.manifest.alias);
        let remote_import_exact = self
            .remote_import
            .revalidate_sealed_repo_removal(repo_id, &removal.manifest.remote_import, &remote_plan)
            .unwrap_or(false);
        if !notegit_exact
            || !authority_exact
            || !locator_exact
            || !alias_exact
            || !remote_import_exact
        {
            return Err(self
                .rollback_owned_removal(
                    watcher,
                    Some(provider),
                    None,
                    Some(&remote_plan),
                    "removal owner identity changed before the durable cut".to_string(),
                )
                .await);
        }
        let revalidated = match self.repo.revalidate_repo_removal_membership(&prepared) {
            Ok(revalidated) => revalidated,
            Err(error) => {
                return Err(self
                    .rollback_owned_removal(
                        watcher,
                        Some(provider),
                        None,
                        Some(&remote_plan),
                        error.to_string(),
                    )
                    .await);
            }
        };
        let authority = match self
            .repo
            .quiesce_local_authority_for_removal(&removal.manifest.authority)
        {
            Ok(authority) => authority,
            Err(error) => {
                return Err(self
                    .rollback_owned_removal(
                        watcher,
                        Some(provider),
                        None,
                        Some(&remote_plan),
                        error.to_string(),
                    )
                    .await);
            }
        };
        removal.state = match removal.progress.cut_attempted().await {
            Ok(state) => state,
            Err(error) => {
                return Err(self
                    .rollback_owned_removal(
                        watcher,
                        Some(provider),
                        Some(authority),
                        Some(&remote_plan),
                        error.to_string(),
                    )
                    .await);
            }
        };
        #[cfg(test)]
        if self.fail_after_cut_attempted.swap(false, Ordering::AcqRel) {
            return Err(RepoLifecycleJobCompletion::repair_required(
                "injected crash after CutAttempted persistence",
            ));
        }
        let cut = self
            .gate
            .execute_catalog_repo_cut(repo_id, |permit| {
                self.repo
                    .commit_repo_removal_membership(&prepared, &revalidated, permit)
            })
            .await;
        let tombstone = match cut {
            Ok(Ok(commit)) => commit.record().clone(),
            other => {
                let primary = match other {
                    Ok(Err(error)) => error.to_string(),
                    Err(error) => error.to_string(),
                    Ok(Ok(_)) => unreachable!("successful cut handled above"),
                };
                match self.repo.repo_catalog_membership_record(repo_id) {
                    Ok(Some(record))
                        if record.confirms_removed_manifest(
                            removal.execute_request_id,
                            &removal.manifest_digest,
                        ) =>
                    {
                        record
                    }
                    Ok(Some(record)) if record == removal.manifest.catalog => {
                        let completion = self
                            .rollback_owned_removal(
                                watcher,
                                Some(provider),
                                Some(authority),
                                Some(&remote_plan),
                                primary,
                            )
                            .await;
                        if completion.outcome == RepoLifecycleJobOutcome::NotCommitted {
                            removal.state = removal
                                .progress
                                .cut_not_committed()
                                .await
                                .map_err(|error| {
                                    RepoLifecycleJobCompletion::repair_required(format!(
                                        "pre-cut compensation succeeded but CutAttempted could not be cleared: {error}"
                                    ))
                                })?;
                        }
                        return Err(completion);
                    }
                    observed => {
                        std::mem::forget(authority);
                        return Err(RepoLifecycleJobCompletion::repair_required(format!(
                            "remove cut outcome is not uniquely classifiable after {primary}: {observed:?}"
                        )));
                    }
                }
            }
        };

        #[cfg(test)]
        if self.fail_after_catalog_cut.swap(false, Ordering::AcqRel) {
            return Err(RepoLifecycleJobCompletion::repair_required(
                "injected crash after catalog cut and before CutObserved persistence",
            ));
        }

        removal.state = match removal.progress.cut_observed(tombstone).await {
            Ok(state) => state,
            Err(error) => {
                std::mem::forget(authority);
                return Err(RepoLifecycleJobCompletion::repair_required(format!(
                    "catalog removal committed but CutObserved could not be persisted: {error}"
                )));
            }
        };

        let cleanup_guard = match authority.into_committed_cleanup() {
            Ok(guard) => guard,
            Err(error) => {
                return Err(RepoLifecycleJobCompletion::repair_required(format!(
                    "catalog removal committed but authority close failed: {error}"
                )));
            }
        };
        Ok((
            cleanup_guard,
            Some(LiveRemovalReservation { watcher, provider }),
        ))
    }

    pub(super) fn static_removal_owners_are_exact(&self, removal: &RepoRemovalExecution) -> bool {
        let repo_id = removal.manifest.repo_id;
        removal.manifest.notegit.revalidate().unwrap_or(false)
            && removal.manifest.authority.revalidate().unwrap_or(false)
            && self
                .repo
                .prepare_projection_locator_removal(repo_id)
                .is_ok_and(|plan| plan == removal.manifest.locator)
            && self
                .repo
                .host_repo_alias_runtime()
                .prepare_removal(repo_id)
                .is_ok_and(|plan| plan == removal.manifest.alias)
    }
}

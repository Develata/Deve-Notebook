//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 04_repository#remote-import-repo-lifecycle
//!   - 03_storage/watcher#watcher-contract
//!
//! Sole host orchestrator for dynamic local-repo create/rename/remove. It
//! keeps Catalog -> Repo critical sections short and owns no durable facts.

mod admission;
mod io;
mod mount;
#[cfg(test)]
mod tests;
mod types;

pub(crate) use types::{
    CreateRepoIntent, CreateRepoOutcome, RemoveRepoOutcome, RepoLifecycleError, RepoMountOutcome,
    RepoRemovalFallback,
};

use crate::remote_import_runtime::RemoteImportCoordinator;
use crate::repo_init::prepare_local_repo_workspace_with_owner;
use crate::server::repo_mutation::RepoMutationPublicationGate;
use crate::server::runtime::watcher_runtime::{WatcherMountReservation, WatcherSupervisor};
use deve_core::ledger::{CatalogMembershipRuntime, LocalRepoSummary};
use deve_core::models::RepoId;
use deve_core::remote_import::{
    RemoteImportRepoRemovalRevalidation, RemoteImportRepoRemovalSnapshot,
};
use deve_core::sync::SyncManager;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::AtomicBool;

pub(crate) struct RepoLifecycleCoordinator {
    repo: Arc<deve_core::ledger::RepoManager>,
    sync: Arc<SyncManager>,
    gate: Arc<RepoMutationPublicationGate>,
    watchers: Arc<WatcherSupervisor>,
    remote_import: Arc<RemoteImportCoordinator>,
    membership: CatalogMembershipRuntime,
    #[cfg(test)]
    fail_fallback_publication: AtomicBool,
}

struct RemoveReservation {
    watcher: WatcherMountReservation,
    old: LocalRepoSummary,
    fallback: Option<RepoRemovalFallback>,
    import_snapshot: RemoteImportRepoRemovalSnapshot,
}

enum RemoveCutError {
    Precondition(RepoLifecycleError),
    Catalog(deve_core::ledger::RepoCatalogError),
}

impl RepoLifecycleCoordinator {
    pub(crate) fn new(
        repo: Arc<deve_core::ledger::RepoManager>,
        sync: Arc<SyncManager>,
        gate: Arc<RepoMutationPublicationGate>,
        watchers: Arc<WatcherSupervisor>,
        remote_import: Arc<RemoteImportCoordinator>,
        membership: CatalogMembershipRuntime,
    ) -> Arc<Self> {
        Arc::new(Self {
            repo,
            sync,
            gate,
            watchers,
            remote_import,
            membership,
            #[cfg(test)]
            fail_fallback_publication: AtomicBool::new(false),
        })
    }

    #[cfg(test)]
    pub(crate) fn shutdown_watchers_for_test(&self) {
        if let Err(error) = self.watchers.shutdown() {
            tracing::warn!(%error, "test repo lifecycle watcher shutdown failed");
        }
    }

    #[cfg(test)]
    pub(crate) fn fail_next_watcher_start_for_test(&self) {
        self.watchers.fail_next_start_for_test();
    }

    #[cfg(test)]
    pub(crate) fn fail_next_watcher_shutdown_after_cleanup_for_test(&self) {
        self.watchers.fail_next_shutdown_after_cleanup_for_test();
    }

    #[cfg(test)]
    pub(crate) fn watcher_is_mounted_for_test(&self, repo_id: RepoId) -> bool {
        self.watchers.mounted_generation(repo_id).is_ok()
    }

    pub(crate) async fn create(
        &self,
        intent: CreateRepoIntent,
    ) -> Result<CreateRepoOutcome, RepoLifecycleError> {
        let execution_name = intent.repo_id.to_string();
        let reservation = self
            .gate
            .execute_catalog_repo_unpublished(intent.repo_id, || {
                if self
                    .repo
                    .repo_catalog_membership_record(intent.repo_id)?
                    .is_some()
                {
                    return Err(RepoLifecycleError::NotCommitted {
                        operation: "create",
                        detail: format!("repository already exists: {}", intent.repo_id),
                    });
                }
                Ok(self.watchers.reserve_new(intent.repo_id)?)
            })
            .await??;

        let prepared_workspace = match prepare_local_repo_workspace_with_owner(
            &self.repo,
            intent.repo_id,
            &intent.projection_base,
            None,
        ) {
            Ok(report) => report,
            Err(primary) => {
                return Err(self.abort_create_pre_cut(
                    reservation,
                    intent.repo_id,
                    "create prepare",
                    primary.to_string(),
                ));
            }
        };
        let prepared = match self.repo.prepare_repo_creation_membership_with_authority(
            intent.repo_id,
            intent.lifecycle_request_id,
            &prepared_workspace,
        ) {
            Ok(prepared) => prepared,
            Err(error) => {
                return Err(self.abort_create_pre_cut(
                    reservation,
                    intent.repo_id,
                    "create membership prepare",
                    error.to_string(),
                ));
            }
        };
        let revalidated = match self
            .repo
            .revalidate_repo_creation_membership_with_authority(&prepared, &prepared_workspace)
        {
            Ok(revalidated) => revalidated,
            Err(error) => {
                return Err(self.abort_create_pre_cut(
                    reservation,
                    intent.repo_id,
                    "create membership revalidate",
                    error.to_string(),
                ));
            }
        };
        let cut = self
            .gate
            .execute_catalog_repo_cut(intent.repo_id, |permit| {
                self.repo
                    .commit_repo_creation_membership(&prepared, &revalidated, permit)
            })
            .await;
        let commit = match cut {
            Ok(Ok(commit)) => commit,
            first => {
                let (phase, primary) = match first {
                    Ok(Err(error)) => ("create catalog cut", error.to_string()),
                    Err(error) => ("create catalog gate", error.to_string()),
                    Ok(Ok(_)) => unreachable!("success handled by outer match"),
                };
                let committed = self
                    .repo
                    .repo_catalog_membership_record(intent.repo_id)
                    .ok()
                    .flatten()
                    .is_some_and(|record| record.confirms_created(intent.lifecycle_request_id));
                if !committed {
                    return Err(self.abort_create_pre_cut(
                        reservation,
                        intent.repo_id,
                        phase,
                        primary,
                    ));
                }
                let retry = self
                    .gate
                    .execute_catalog_repo_cut(intent.repo_id, |permit| {
                        self.repo
                            .commit_repo_creation_membership(&prepared, &revalidated, permit)
                    })
                    .await;
                match retry {
                    Ok(Ok(commit)) => commit,
                    retry => {
                        let retry = match retry {
                            Ok(Err(error)) => error.to_string(),
                            Err(error) => error.to_string(),
                            Ok(Ok(_)) => unreachable!("success handled by outer match"),
                        };
                        let detail =
                            format!("{primary}; exact committed create retry failed: {retry}");
                        mount::mark_repair_required(&self.watchers, &reservation, detail.clone());
                        return Err(RepoLifecycleError::RepairRequired {
                            operation: "create authority activation",
                            repo_id: intent.repo_id,
                            detail,
                        });
                    }
                }
            }
        };
        debug_assert_eq!(commit.record().repo_id(), intent.repo_id);

        if let Err(error) =
            self.repo
                .activate_prepared_local_repo_authority(prepared_workspace, &prepared, &commit)
        {
            let detail =
                format!("catalog committed but local authority activation failed: {error}");
            mount::mark_repair_required(&self.watchers, &reservation, detail.clone());
            return Err(RepoLifecycleError::RepairRequired {
                operation: "create authority activation",
                repo_id: intent.repo_id,
                detail,
            });
        }

        if let Err(error) =
            self.repo
                .host_repo_alias_runtime()
                .set_alias(intent.repo_id, &intent.initial_alias, 0)
        {
            tracing::error!(repo_id = %intent.repo_id, %error, "repo create committed but initial host alias settlement failed");
        }
        let mount = self.mount(reservation, execution_name).await?;
        Ok(CreateRepoOutcome { mount })
    }

    pub(crate) async fn remove(
        &self,
        repo_id: RepoId,
        lifecycle_request_id: uuid::Uuid,
    ) -> Result<RemoveRepoOutcome, RepoLifecycleError> {
        let admission = self.remote_import.repo_removal_admission(repo_id)?;
        let import_snapshot = admission::admitted_snapshot(admission)?;
        let initial = self
            .gate
            .execute_catalog_repo_unpublished(repo_id, || {
                match self
                    .remote_import
                    .revalidate_repo_removal(repo_id, &import_snapshot)?
                {
                    RemoteImportRepoRemovalRevalidation::Exact => {}
                    RemoteImportRepoRemovalRevalidation::Changed(current) => {
                        return Err(admission::admission_error(current));
                    }
                }
                let summaries = self.repo.list_cataloged_local_repo_summaries()?;
                if summaries.len() <= 1 {
                    return Err(RepoLifecycleError::NotCommitted {
                        operation: "remove",
                        detail: "cannot remove the last local repository".to_string(),
                    });
                }
                let old = summaries
                    .iter()
                    .find(|summary| summary.repo_id == repo_id)
                    .cloned()
                    .ok_or_else(|| RepoLifecycleError::NotCommitted {
                        operation: "remove",
                        detail: format!("local repo not found for UUID {repo_id}"),
                    })?;
                let mut fallback = None;
                for summary in summaries
                    .into_iter()
                    .filter(|summary| summary.repo_id != repo_id)
                {
                    let Ok(mount) = self.gate.admit_mounted_repo(summary.repo_id) else {
                        continue;
                    };
                    let membership = self.membership.issue(summary.repo_id)?;
                    fallback = Some(RepoRemovalFallback::new(summary, membership, mount));
                    break;
                }
                if fallback.is_none() {
                    return Err(RepoLifecycleError::NotCommitted {
                        operation: "remove",
                        detail: "no alternate Healthy + Mounted local repository".to_string(),
                    });
                }
                Ok(RemoveReservation {
                    watcher: self.watchers.reserve_existing(repo_id)?,
                    old,
                    fallback,
                    import_snapshot,
                })
            })
            .await??;
        let prepared = match self
            .repo
            .prepare_repo_removal_membership(repo_id, lifecycle_request_id)
        {
            Ok(prepared) => prepared,
            Err(error) => {
                return Err(self.cancel_remove_before_stop(initial, error.to_string()));
            }
        };
        let initial = self.stop_remove(initial).await?;
        let provider = match self.quiesce_provider(repo_id).await {
            Ok(provider) => provider,
            Err(error) => {
                return Err(self
                    .restore_remove_pre_cut(initial, None, error.to_string())
                    .await);
            }
        };
        let import_revalidation = match self
            .remote_import
            .revalidate_repo_removal(repo_id, &initial.import_snapshot)
        {
            Ok(revalidation) => revalidation,
            Err(error) => {
                return Err(self
                    .restore_remove_pre_cut(initial, Some(provider), error.to_string())
                    .await);
            }
        };
        match import_revalidation {
            RemoteImportRepoRemovalRevalidation::Exact => {}
            RemoteImportRepoRemovalRevalidation::Changed(current) => {
                let primary = admission::admission_error(current);
                return Err(self
                    .restore_remove_pre_cut(initial, Some(provider), primary.to_string())
                    .await);
            }
        }
        if let Some(fallback) = initial.fallback.as_ref()
            && let Err(error) = fallback.revalidate_outside_cut(&self.repo)
        {
            return Err(self
                .restore_remove_pre_cut(initial, Some(provider), error.to_string())
                .await);
        }
        let revalidated = match self.repo.revalidate_repo_removal_membership(&prepared) {
            Ok(revalidated) => revalidated,
            Err(error) => {
                return Err(self
                    .restore_remove_pre_cut(initial, Some(provider), error.to_string())
                    .await);
            }
        };
        let cut = match self
            .gate
            .execute_catalog_repo_cut(repo_id, |permit| {
                if let Some(fallback) = initial.fallback.as_ref() {
                    fallback
                        .revalidate_cut(&self.membership)
                        .map_err(RemoveCutError::Precondition)?;
                }
                self.repo
                    .commit_repo_removal_membership(&prepared, &revalidated, permit)
                    .map_err(RemoveCutError::Catalog)
            })
            .await
        {
            Ok(cut) => cut,
            Err(error) => {
                return Err(self
                    .restore_remove_pre_cut(initial, Some(provider), error.to_string())
                    .await);
            }
        };
        let _cut = match cut {
            Ok(cut) => cut,
            Err(RemoveCutError::Catalog(error)) if error.cut_may_be_committed() => {
                let retry = match self
                    .gate
                    .execute_catalog_repo_cut(repo_id, |permit| {
                        if let Some(fallback) = initial.fallback.as_ref() {
                            fallback
                                .revalidate_cut(&self.membership)
                                .map_err(RemoveCutError::Precondition)?;
                        }
                        self.repo
                            .commit_repo_removal_membership(&prepared, &revalidated, permit)
                            .map_err(RemoveCutError::Catalog)
                    })
                    .await
                {
                    Ok(retry) => retry,
                    Err(retry) => {
                        let detail = format!("{error}; exact cut retry gate failed: {retry}");
                        self.fail_reservation(initial.watcher, detail.clone()).await;
                        return Err(RepoLifecycleError::RepairRequired {
                            operation: "remove cut",
                            repo_id,
                            detail,
                        });
                    }
                };
                match retry {
                    Ok(cut) => cut,
                    Err(RemoveCutError::Catalog(retry)) => {
                        let detail = format!("{error}; exact cut retry failed: {retry}");
                        self.fail_reservation(initial.watcher, detail.clone()).await;
                        return Err(RepoLifecycleError::RepairRequired {
                            operation: "remove cut",
                            repo_id,
                            detail,
                        });
                    }
                    Err(RemoveCutError::Precondition(retry)) => {
                        let detail =
                            format!("{error}; exact cut retry precondition failed: {retry}");
                        self.fail_reservation(initial.watcher, detail.clone()).await;
                        return Err(RepoLifecycleError::RepairRequired {
                            operation: "remove cut",
                            repo_id,
                            detail,
                        });
                    }
                }
            }
            Err(RemoveCutError::Catalog(error)) => {
                return Err(self
                    .restore_remove_pre_cut(initial, Some(provider), error.to_string())
                    .await);
            }
            Err(RemoveCutError::Precondition(error)) => {
                return Err(self
                    .restore_remove_pre_cut(initial, Some(provider), error.to_string())
                    .await);
            }
        };

        let mut repair_required = false;
        if let Err(error) = self.repo.remove_projection_locator_for_repo_id(repo_id) {
            tracing::error!(repo_id = %repo_id, %error, "removed repo locator cleanup failed");
            repair_required = true;
        }
        if let Err(error) = self.finish_provider(provider).await {
            tracing::error!(%error, "removed repo provider finalization failed");
            repair_required = true;
        }
        if let Err(error) = self.watchers.finalize_removed(initial.watcher) {
            tracing::error!(repo_id = %repo_id, %error, "removed repo watcher finalization failed");
            repair_required = true;
        }
        #[cfg(test)]
        self.fail_fallback_publication_for_test(initial.fallback.as_ref());
        Ok(RemoveRepoOutcome {
            fallback: initial.fallback,
            repair_required,
        })
    }
}

impl From<anyhow::Error> for RepoLifecycleError {
    fn from(error: anyhow::Error) -> Self {
        Self::NotCommitted {
            operation: "lifecycle",
            detail: error.to_string(),
        }
    }
}

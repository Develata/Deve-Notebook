//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! Blocking host I/O adapters kept outside mutation permits.

#[cfg(test)]
use super::RepoRemovalFallback;
use super::{RemoveReservation, RepoLifecycleCoordinator};
use crate::remote_import_runtime::ProviderQuiesceToken;
use crate::server::runtime::watcher_runtime::WatcherMountReservation;
use deve_core::models::RepoId;
#[cfg(test)]
use deve_core::sync::watcher::{WatcherFailure, WatcherFailureKind, WatcherFailurePhase};
#[cfg(test)]
use std::sync::atomic::Ordering;

impl RepoLifecycleCoordinator {
    pub(super) fn abort_create_pre_cut(
        &self,
        reservation: WatcherMountReservation,
        repo_id: RepoId,
        operation: &'static str,
        primary: impl Into<String>,
    ) -> super::RepoLifecycleError {
        let primary = primary.into();
        let detail = match self.watchers.cancel_unstarted(reservation) {
            Ok(()) => primary,
            Err(cleanup) => {
                format!("{primary}; watcher reservation cleanup also failed: {cleanup}")
            }
        };
        super::RepoLifecycleError::RepairRequired {
            operation,
            repo_id,
            detail,
        }
    }

    pub(super) fn cancel_remove_before_stop(
        &self,
        initial: RemoveReservation,
        primary: impl Into<String>,
    ) -> super::RepoLifecycleError {
        let repo_id = initial.old.repo_id;
        let primary = primary.into();
        match self.watchers.cancel_unstarted(initial.watcher) {
            Ok(()) => super::RepoLifecycleError::NotCommitted {
                operation: "remove",
                detail: primary,
            },
            Err(cleanup) => super::RepoLifecycleError::RepairRequired {
                operation: "remove pre-cut cleanup",
                repo_id,
                detail: format!("{primary}; watcher reservation cleanup also failed: {cleanup}"),
            },
        }
    }

    pub(super) async fn restore_remove_pre_cut(
        &self,
        initial: RemoveReservation,
        provider: Option<ProviderQuiesceToken>,
        primary: impl Into<String>,
    ) -> super::RepoLifecycleError {
        let repo_id = initial.old.repo_id;
        let primary = primary.into();
        let mut cleanup = Vec::new();
        if let Some(provider) = provider
            && let Err(error) = self.resume_provider(provider).await
        {
            cleanup.push(format!("provider resume failed: {error}"));
        }
        match self
            .mount(initial.watcher, initial.old.execution_name)
            .await
        {
            Ok(outcome) if outcome.is_mounted() => {}
            Ok(_) => cleanup.push("watcher remount did not reach Mounted".to_string()),
            Err(error) => cleanup.push(format!("watcher remount failed: {error}")),
        }
        if cleanup.is_empty() {
            super::RepoLifecycleError::NotCommitted {
                operation: "remove",
                detail: primary,
            }
        } else {
            super::RepoLifecycleError::RepairRequired {
                operation: "remove pre-cut cleanup",
                repo_id,
                detail: format!("{primary}; cleanup: {}", cleanup.join("; ")),
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn fail_fallback_before_publication_for_test(&self) {
        self.fail_fallback_publication
            .store(true, Ordering::Release);
    }

    #[cfg(test)]
    pub(super) fn fail_fallback_publication_for_test(
        &self,
        fallback: Option<&RepoRemovalFallback>,
    ) {
        if !self.fail_fallback_publication.swap(false, Ordering::AcqRel) {
            return;
        }
        let Some(fallback) = fallback else {
            return;
        };
        let repo_id = fallback.summary().repo_id;
        let Ok(generation) = self.watchers.mounted_generation(repo_id) else {
            return;
        };
        let _ = self.watchers.fail_for_test(
            repo_id,
            generation,
            WatcherFailure::new(
                WatcherFailurePhase::Worker,
                WatcherFailureKind::Repository,
                "injected fallback failure before publication",
            ),
        );
    }

    pub(super) async fn stop_remove(
        &self,
        initial: RemoveReservation,
    ) -> Result<RemoveReservation, super::RepoLifecycleError> {
        let watchers = self.watchers.clone();
        let (initial, result) = tokio::task::spawn_blocking(move || {
            let result = super::mount::stop_reserved(&watchers, &initial.watcher);
            (initial, result)
        })
        .await
        .map_err(|_| super::RepoLifecycleError::Coordination("watcher stop task failed"))?;
        match result {
            Ok(()) => Ok(initial),
            Err(error) => Err(self
                .restore_remove_pre_cut(initial, None, error.to_string())
                .await),
        }
    }

    pub(super) async fn mount(
        &self,
        reservation: WatcherMountReservation,
        execution_name: String,
    ) -> Result<super::RepoMountOutcome, super::RepoLifecycleError> {
        let watchers = self.watchers.clone();
        let sync = self.sync.clone();
        tokio::task::spawn_blocking(move || {
            super::mount::mount_reserved(&watchers, sync, &reservation, execution_name)
        })
        .await
        .map_err(|_| super::RepoLifecycleError::Coordination("watcher mount task failed"))
    }

    pub(super) async fn fail_reservation(
        &self,
        reservation: WatcherMountReservation,
        detail: String,
    ) {
        let watchers = self.watchers.clone();
        let _ = tokio::task::spawn_blocking(move || {
            super::mount::mark_repair_required(&watchers, &reservation, detail)
        })
        .await;
    }

    pub(super) async fn quiesce_provider(
        &self,
        repo_id: RepoId,
    ) -> Result<ProviderQuiesceToken, super::RepoLifecycleError> {
        let remote_import = self.remote_import.clone();
        tokio::task::spawn_blocking(move || remote_import.quiesce_provider_for_remove(repo_id))
            .await
            .map_err(|_| super::RepoLifecycleError::Coordination("provider quiesce task failed"))?
            .map_err(Into::into)
    }

    pub(super) async fn resume_provider(
        &self,
        token: ProviderQuiesceToken,
    ) -> Result<(), super::RepoLifecycleError> {
        let remote_import = self.remote_import.clone();
        tokio::task::spawn_blocking(move || {
            remote_import.resume_provider_after_failed_remove(&token)
        })
        .await
        .map_err(|_| super::RepoLifecycleError::Coordination("provider resume task failed"))??;
        Ok(())
    }

    pub(super) async fn finish_provider(
        &self,
        token: ProviderQuiesceToken,
    ) -> Result<(), super::RepoLifecycleError> {
        let remote_import = self.remote_import.clone();
        tokio::task::spawn_blocking(move || remote_import.finish_provider_after_remove(token))
            .await
            .map_err(|_| {
                super::RepoLifecycleError::Coordination("provider finish task failed")
            })??;
        Ok(())
    }
}

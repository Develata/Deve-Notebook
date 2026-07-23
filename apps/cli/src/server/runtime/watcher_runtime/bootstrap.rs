//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 04_repository#repo-health-and-repair
//!   - 07_network#server-ws-runtime
//!
//! Bootstrap isolation policy. Repo-local failures settle one failed slot;
//! only typed host-fatal failures roll back the complete watcher collection.

mod classification;

use self::classification::{
    BootstrapFailure, append_cleanup, classify_bootstrap_error, classify_host_lifecycle_error,
};
use super::error::{WatcherHostFatalKind, WatcherSupervisorStartError};
use super::lifecycle::WatcherMountReservation;
use super::slot::RepoMountState;
use super::supervisor::WatcherSupervisor;
use deve_core::models::RepoId;
use deve_core::sync::watcher::{RepoWatcherStart, WatcherFailure, WatcherRefreshCallback};
use std::collections::HashSet;

impl WatcherSupervisor {
    pub(crate) fn start_all(
        starts: Vec<RepoWatcherStart>,
        publisher: WatcherRefreshCallback,
    ) -> Result<Self, WatcherSupervisorStartError> {
        Self::start_all_inner(starts, publisher, |_, _| None)
    }

    fn start_all_inner<F>(
        starts: Vec<RepoWatcherStart>,
        publisher: WatcherRefreshCallback,
        mut before_start: F,
    ) -> Result<Self, WatcherSupervisorStartError>
    where
        F: FnMut(usize, &Self) -> Option<WatcherSupervisorStartError>,
    {
        let mut repo_ids = HashSet::with_capacity(starts.len());
        for start in &starts {
            if !repo_ids.insert(start.repo_id()) {
                return Err(WatcherSupervisorStartError::new(
                    WatcherHostFatalKind::SupervisorInvariant,
                    Some(start.repo_id()),
                    "duplicate watcher bootstrap reservation",
                ));
            }
        }

        let supervisor = Self::empty(starts.len(), publisher);
        let mut expected = Vec::with_capacity(starts.len());
        for (index, start) in starts.into_iter().enumerate() {
            let repo_id = start.repo_id();
            let generation = start.generation();
            if let Some(error) = before_start(index, &supervisor) {
                return Err(supervisor.rollback_host_fatal(error));
            }
            expected.push((repo_id, generation));
            let reservation = match supervisor.reserve_bootstrap(repo_id, generation) {
                Ok(reservation) => reservation,
                Err(error) => {
                    return Err(supervisor
                        .rollback_host_fatal(classify_host_lifecycle_error(repo_id, error)));
                }
            };
            if let Err(error) = supervisor.start_reserved_inner(&reservation, start) {
                match classify_bootstrap_error(repo_id, error) {
                    BootstrapFailure::RepoLocal(failure) => {
                        if let Err(error) =
                            supervisor.settle_repo_local_bootstrap_failure(&reservation, failure)
                        {
                            return Err(supervisor.rollback_host_fatal(error));
                        }
                        continue;
                    }
                    BootstrapFailure::HostFatal(error) => {
                        return Err(supervisor.rollback_host_fatal(error));
                    }
                }
            }
            if let Err(error) = supervisor.finalize_mounted(&reservation) {
                match classify_bootstrap_error(repo_id, error) {
                    BootstrapFailure::RepoLocal(failure) => {
                        if let Err(error) =
                            supervisor.settle_repo_local_bootstrap_failure(&reservation, failure)
                        {
                            return Err(supervisor.rollback_host_fatal(error));
                        }
                    }
                    BootstrapFailure::HostFatal(error) => {
                        return Err(supervisor.rollback_host_fatal(error));
                    }
                }
            }
        }

        if let Err(error) = supervisor.validate_bootstrap(&expected) {
            return Err(supervisor.rollback_host_fatal(error));
        }
        Ok(supervisor)
    }

    #[cfg(test)]
    pub(crate) fn start_all_with_host_fatal_before_for_test(
        starts: Vec<RepoWatcherStart>,
        publisher: WatcherRefreshCallback,
        fail_before_index: usize,
        observed: std::sync::Arc<std::sync::Mutex<Option<super::WatcherRuntimeView>>>,
    ) -> Result<Self, WatcherSupervisorStartError> {
        Self::start_all_inner(starts, publisher, move |index, supervisor| {
            if index != fail_before_index {
                return None;
            }
            *observed.lock().expect("watcher bootstrap observer") = Some(supervisor.view());
            Some(WatcherSupervisorStartError::new(
                WatcherHostFatalKind::ThreadResourceExhaustion,
                None,
                "injected typed host-fatal before watcher start",
            ))
        })
    }

    fn validate_bootstrap(
        &self,
        expected: &[(RepoId, u64)],
    ) -> Result<(), WatcherSupervisorStartError> {
        let slots = self.slots.lock().map_err(|_| {
            WatcherSupervisorStartError::new(
                WatcherHostFatalKind::RuntimeCoordinationFailure,
                expected.first().map(|(repo_id, _)| *repo_id),
                "watcher supervisor slot registry poisoned during bootstrap validation",
            )
        })?;
        if slots.len() != expected.len() {
            return Err(WatcherSupervisorStartError::new(
                WatcherHostFatalKind::SupervisorInvariant,
                None,
                "watcher bootstrap slot set does not match the expected repository set",
            ));
        }
        for (repo_id, generation) in expected {
            let Some(slot) = slots.get(repo_id) else {
                return Err(WatcherSupervisorStartError::new(
                    WatcherHostFatalKind::SupervisorInvariant,
                    Some(*repo_id),
                    "watcher bootstrap slot disappeared before handoff",
                ));
            };
            if slot.generation() != *generation {
                return Err(WatcherSupervisorStartError::new(
                    WatcherHostFatalKind::GenerationCorruption,
                    Some(*repo_id),
                    format!(
                        "watcher bootstrap expected generation {generation}, found {}",
                        slot.generation()
                    ),
                ));
            }
            match slot.state() {
                RepoMountState::Mounted => {}
                RepoMountState::Failed if slot.recorded_failure().is_some() => {}
                state => {
                    return Err(WatcherSupervisorStartError::new(
                        WatcherHostFatalKind::SupervisorInvariant,
                        Some(*repo_id),
                        format!("watcher bootstrap final cut found invalid {state:?} mount state"),
                    ));
                }
            }
        }
        Ok(())
    }

    fn settle_repo_local_bootstrap_failure(
        &self,
        reservation: &WatcherMountReservation,
        mut failure: WatcherFailure,
    ) -> Result<(), WatcherSupervisorStartError> {
        let handle = self
            .take_handle(reservation.repo_id, reservation.generation)
            .map_err(|error| classify_host_lifecycle_error(reservation.repo_id, error))?;
        if let Some(handle) = handle
            && let Err(cleanup) = handle.shutdown()
        {
            append_cleanup(&mut failure, cleanup);
        }
        reservation.target.mark_failed_and_drop(failure.clone());
        tracing::error!(
            repo_id = %reservation.repo_id,
            generation = reservation.generation,
            watcher_phase = ?failure.phase,
            watcher_failure_kind = ?failure.kind,
            primary = %failure.primary,
            cleanup = ?failure.cleanup,
            "repo-local watcher failed during bootstrap; host remains available"
        );
        Ok(())
    }

    fn rollback_host_fatal(
        &self,
        error: WatcherSupervisorStartError,
    ) -> WatcherSupervisorStartError {
        let kind = error.kind();
        let repo_id = error.repo_id();
        let error = error.with_cleanup(self.shutdown_collect());
        tracing::error!(
            watcher_host_fatal_kind = ?kind,
            repo_id = ?repo_id,
            error = %error,
            "host-fatal watcher bootstrap failure rolled back all started watchers"
        );
        error
    }
}

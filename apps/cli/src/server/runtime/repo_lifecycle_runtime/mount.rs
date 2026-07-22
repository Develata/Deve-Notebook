//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! Blocking watcher I/O helpers. Callers invoke these only after releasing
//! every Catalog/Repo mutation permit.

use super::types::RepoMountOutcome;
use crate::server::runtime::watcher_runtime::{
    WatcherLifecycleError, WatcherMountReservation, WatcherSupervisor,
};
use crate::server::setup;
use deve_core::sync::SyncManager;
use deve_core::sync::watcher::{WatcherFailure, WatcherFailureKind, WatcherFailurePhase};
use std::sync::Arc;

#[cfg(test)]
pub(super) fn stop_reserved(
    supervisor: &WatcherSupervisor,
    reservation: &WatcherMountReservation,
) -> Result<(), WatcherLifecycleError> {
    supervisor.shutdown_reserved(reservation)
}

pub(super) fn mount_reserved(
    supervisor: &WatcherSupervisor,
    sync: Arc<SyncManager>,
    reservation: &WatcherMountReservation,
    execution_name: String,
) -> RepoMountOutcome {
    let start = match setup::file_watcher_start(sync, execution_name, reservation.generation()) {
        Ok(start) => start,
        Err(error) => {
            fail_reservation(
                supervisor,
                reservation,
                WatcherFailure::new(
                    WatcherFailurePhase::Prepare,
                    WatcherFailureKind::Repository,
                    error.to_string(),
                ),
            );
            return RepoMountOutcome::Failed;
        }
    };
    if let Err(error) = supervisor.start_reserved(reservation, start) {
        fail_reservation(supervisor, reservation, lifecycle_failure(&error));
        tracing::error!(%error, "dynamic repo watcher start failed");
        return RepoMountOutcome::Failed;
    }
    match supervisor.finalize_mounted(reservation) {
        Ok(_) => RepoMountOutcome::Mounted,
        Err(error) => {
            fail_reservation(supervisor, reservation, lifecycle_failure(&error));
            tracing::error!(%error, "dynamic repo watcher mount finalization failed");
            RepoMountOutcome::Failed
        }
    }
}

pub(super) fn mark_repair_required(
    supervisor: &WatcherSupervisor,
    reservation: &WatcherMountReservation,
    detail: impl Into<String>,
) {
    fail_reservation(
        supervisor,
        reservation,
        WatcherFailure::new(
            WatcherFailurePhase::Reconcile,
            WatcherFailureKind::Repository,
            detail,
        ),
    );
}

fn fail_reservation(
    supervisor: &WatcherSupervisor,
    reservation: &WatcherMountReservation,
    failure: WatcherFailure,
) {
    match supervisor.finalize_failed(reservation, failure.clone()) {
        Ok(snapshot) => {
            debug_assert_eq!(snapshot.repo_id(), reservation.repo_id());
            debug_assert!(snapshot.failure().is_some());
        }
        Err(cleanup) => {
            tracing::error!(primary = %failure, cleanup = %cleanup, "watcher lifecycle failure cleanup also failed");
        }
    }
}

fn lifecycle_failure(error: &WatcherLifecycleError) -> WatcherFailure {
    match error {
        WatcherLifecycleError::Start { source, .. } => source.failure().clone(),
        WatcherLifecycleError::FailedBeforeMounted { failure, .. } => failure.as_ref().clone(),
        WatcherLifecycleError::Shutdown { failure, .. } => failure.clone(),
        _ => WatcherFailure::new(
            WatcherFailurePhase::Reconcile,
            WatcherFailureKind::Coordination,
            error.to_string(),
        ),
    }
}

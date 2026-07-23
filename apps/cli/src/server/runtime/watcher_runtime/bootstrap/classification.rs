//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 04_repository#repo-health-and-repair
//!
//! Exhaustive bootstrap failure classification. Product text is diagnostics
//! only and never participates in host-fatal policy.

use super::super::error::{
    WatcherHostFatalKind, WatcherLifecycleError, WatcherSupervisorStartError,
};
use deve_core::models::RepoId;
use deve_core::sync::watcher::{WatcherFailure, WatcherFailureKind};

pub(super) enum BootstrapFailure {
    RepoLocal(WatcherFailure),
    HostFatal(WatcherSupervisorStartError),
}

pub(super) fn classify_bootstrap_error(
    repo_id: RepoId,
    error: WatcherLifecycleError,
) -> BootstrapFailure {
    match error {
        WatcherLifecycleError::Start { source, .. } => {
            classify_watcher_failure(repo_id, source.into_failure())
        }
        WatcherLifecycleError::FailedBeforeMounted {
            failure, cleanup, ..
        } => {
            let mut failure = *failure;
            if let Some(cleanup) = cleanup {
                append_cleanup(&mut failure, *cleanup);
            }
            classify_watcher_failure(repo_id, failure)
        }
        WatcherLifecycleError::Shutdown { failure, .. } => {
            classify_watcher_failure(repo_id, failure)
        }
        error => BootstrapFailure::HostFatal(classify_host_lifecycle_error(repo_id, error)),
    }
}

fn classify_watcher_failure(repo_id: RepoId, failure: WatcherFailure) -> BootstrapFailure {
    if failure.kind == WatcherFailureKind::Thread {
        BootstrapFailure::HostFatal(WatcherSupervisorStartError::from_failure(
            WatcherHostFatalKind::ThreadResourceExhaustion,
            repo_id,
            failure,
        ))
    } else {
        BootstrapFailure::RepoLocal(failure)
    }
}

pub(super) fn classify_host_lifecycle_error(
    repo_id: RepoId,
    error: WatcherLifecycleError,
) -> WatcherSupervisorStartError {
    let kind = match &error {
        WatcherLifecycleError::GenerationExhausted(_)
        | WatcherLifecycleError::StaleReservation { .. }
        | WatcherLifecycleError::StartIdentity { .. } => WatcherHostFatalKind::GenerationCorruption,
        WatcherLifecycleError::HostCoordination { .. } | WatcherLifecycleError::Coordination(_) => {
            WatcherHostFatalKind::RuntimeCoordinationFailure
        }
        WatcherLifecycleError::AlreadyReserved { .. }
        | WatcherLifecycleError::Busy { .. }
        | WatcherLifecycleError::Missing(_)
        | WatcherLifecycleError::HandleStillOwned { .. } => {
            WatcherHostFatalKind::SupervisorInvariant
        }
        WatcherLifecycleError::Start { .. }
        | WatcherLifecycleError::FailedBeforeMounted { .. }
        | WatcherLifecycleError::Shutdown { .. } => WatcherHostFatalKind::SupervisorInvariant,
    };
    WatcherSupervisorStartError::new(kind, Some(repo_id), error.to_string())
}

pub(super) fn append_cleanup(failure: &mut WatcherFailure, cleanup: WatcherFailure) {
    failure.cleanup.push(cleanup.to_string());
    failure.cleanup.extend(cleanup.cleanup);
}

#[cfg(test)]
mod tests {
    use super::*;
    use deve_core::sync::watcher::WatcherFailurePhase;

    #[test]
    fn bootstrap_failure_taxonomy_never_uses_primary_text() {
        let repo_id = RepoId::new_v4();
        let local = classify_watcher_failure(
            repo_id,
            WatcherFailure::new(
                WatcherFailurePhase::Attach,
                WatcherFailureKind::Backend,
                "thread resource exhaustion",
            ),
        );
        assert!(matches!(local, BootstrapFailure::RepoLocal(_)));

        let host_fatal = classify_watcher_failure(
            repo_id,
            WatcherFailure::new(
                WatcherFailurePhase::WorkerSpawn,
                WatcherFailureKind::Thread,
                "ordinary spawn failure",
            ),
        );
        assert!(matches!(
            host_fatal,
            BootstrapFailure::HostFatal(error)
                if error.kind() == WatcherHostFatalKind::ThreadResourceExhaustion
                    && error.repo_id() == Some(repo_id)
        ));
    }

    #[test]
    fn core_coordination_failure_remains_repo_local() {
        let repo_id = RepoId::new_v4();
        let failure = classify_watcher_failure(
            repo_id,
            WatcherFailure::new(
                WatcherFailurePhase::Worker,
                WatcherFailureKind::Coordination,
                "repo worker handoff failed",
            ),
        );

        assert!(matches!(failure, BootstrapFailure::RepoLocal(_)));
    }

    #[test]
    fn watcher_server_isolation_host_coordination_is_typed_host_fatal() {
        let repo_id = RepoId::new_v4();
        let failure = classify_bootstrap_error(
            repo_id,
            WatcherLifecycleError::HostCoordination {
                detail: "owner registry unavailable",
                cleanup: Some(Box::new(WatcherFailure::new(
                    WatcherFailurePhase::Shutdown,
                    WatcherFailureKind::Backend,
                    "cleanup failed",
                ))),
            },
        );

        assert!(matches!(
            failure,
            BootstrapFailure::HostFatal(error)
                if error.kind() == WatcherHostFatalKind::RuntimeCoordinationFailure
                    && error.repo_id() == Some(repo_id)
                    && error.to_string().contains("cleanup failed")
        ));
    }
}

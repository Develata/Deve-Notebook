//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 04_repository#repo-health-and-repair
//!
//! Typed host errors for watcher bootstrap and dynamic repo lifecycle.

use super::slot::RepoMountState;
use deve_core::models::RepoId;
use deve_core::sync::watcher::{WatcherFailure, WatcherStartError};
use std::fmt;

#[derive(Debug)]
pub(crate) enum WatcherLifecycleError {
    AlreadyReserved {
        repo_id: RepoId,
        generation: u64,
    },
    #[allow(dead_code)] // R4 ownership-aware remove reserves existing mounts.
    Busy {
        repo_id: RepoId,
        generation: u64,
        state: RepoMountState,
    },
    #[allow(dead_code)] // R4 ownership-aware remove reserves existing mounts.
    Missing(RepoId),
    #[allow(dead_code)] // R4 ownership-aware remove reserves existing mounts.
    GenerationExhausted(RepoId),
    StaleReservation {
        repo_id: RepoId,
        generation: u64,
    },
    StartIdentity {
        repo_id: RepoId,
        generation: u64,
        actual_repo_id: RepoId,
        actual_generation: u64,
    },
    Start {
        repo_id: RepoId,
        source: WatcherStartError,
    },
    FailedBeforeMounted {
        repo_id: RepoId,
        failure: Box<WatcherFailure>,
        cleanup: Option<Box<WatcherFailure>>,
    },
    HandleStillOwned {
        repo_id: RepoId,
        generation: u64,
    },
    #[allow(dead_code)] // R4 ownership-aware remove performs E2 shutdown.
    Shutdown {
        repo_id: RepoId,
        failure: WatcherFailure,
    },
    HostCoordination {
        detail: &'static str,
        cleanup: Option<Box<WatcherFailure>>,
    },
    Coordination(&'static str),
}

impl fmt::Display for WatcherLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyReserved {
                repo_id,
                generation,
            } => write!(
                formatter,
                "watcher slot already exists for repo {repo_id} at generation {generation}"
            ),
            Self::Busy {
                repo_id,
                generation,
                state,
            } => write!(
                formatter,
                "watcher lifecycle is busy for repo {repo_id} at generation {generation} ({state:?})"
            ),
            Self::Missing(repo_id) => write!(formatter, "watcher slot missing for repo {repo_id}"),
            Self::GenerationExhausted(repo_id) => {
                write!(formatter, "watcher generation exhausted for repo {repo_id}")
            }
            Self::StaleReservation {
                repo_id,
                generation,
            } => write!(
                formatter,
                "stale watcher reservation for repo {repo_id} at generation {generation}"
            ),
            Self::StartIdentity {
                repo_id,
                generation,
                actual_repo_id,
                actual_generation,
            } => write!(
                formatter,
                "watcher start identity mismatch: reserved {repo_id}/{generation}, got {actual_repo_id}/{actual_generation}"
            ),
            Self::Start { repo_id, source } => {
                write!(
                    formatter,
                    "watcher start failed for repo {repo_id}: {source}"
                )
            }
            Self::FailedBeforeMounted {
                repo_id,
                failure,
                cleanup,
            } => {
                write!(
                    formatter,
                    "watcher failed before mount finalization for repo {repo_id}: {failure}"
                )?;
                if let Some(cleanup) = cleanup {
                    write!(formatter, "; cleanup failure: {cleanup}")?;
                }
                Ok(())
            }
            Self::HandleStillOwned {
                repo_id,
                generation,
            } => write!(
                formatter,
                "watcher handle still owned for repo {repo_id} at generation {generation}"
            ),
            Self::Shutdown { repo_id, failure } => {
                write!(
                    formatter,
                    "watcher shutdown failed for repo {repo_id}: {failure}"
                )
            }
            Self::HostCoordination { detail, cleanup } => {
                formatter.write_str(detail)?;
                if let Some(cleanup) = cleanup {
                    write!(formatter, "; cleanup failure: {cleanup}")?;
                }
                Ok(())
            }
            Self::Coordination(detail) => formatter.write_str(detail),
        }
    }
}

impl std::error::Error for WatcherLifecycleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Start { source, .. } => Some(source),
            Self::FailedBeforeMounted { failure, .. } => Some(failure.as_ref()),
            Self::Shutdown { failure, .. } => Some(failure),
            Self::HostCoordination {
                cleanup: Some(cleanup),
                ..
            } => Some(cleanup.as_ref()),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WatcherHostFatalKind {
    SupervisorInvariant,
    GenerationCorruption,
    ThreadResourceExhaustion,
    RuntimeCoordinationFailure,
}

#[derive(Debug)]
pub(crate) struct WatcherSupervisorStartError {
    kind: WatcherHostFatalKind,
    repo_id: Option<RepoId>,
    primary: String,
    failure: Option<Box<WatcherFailure>>,
    cleanup: Vec<WatcherFailure>,
}

impl WatcherSupervisorStartError {
    pub(super) fn new(
        kind: WatcherHostFatalKind,
        repo_id: Option<RepoId>,
        primary: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            repo_id,
            primary: primary.into(),
            failure: None,
            cleanup: Vec::new(),
        }
    }

    pub(super) fn from_failure(
        kind: WatcherHostFatalKind,
        repo_id: RepoId,
        failure: WatcherFailure,
    ) -> Self {
        Self {
            kind,
            repo_id: Some(repo_id),
            primary: failure.to_string(),
            failure: Some(Box::new(failure)),
            cleanup: Vec::new(),
        }
    }

    pub(super) fn with_cleanup(mut self, cleanup: Vec<WatcherFailure>) -> Self {
        self.cleanup = cleanup;
        self
    }

    pub(crate) fn kind(&self) -> WatcherHostFatalKind {
        self.kind
    }

    pub(crate) fn repo_id(&self) -> Option<RepoId> {
        self.repo_id
    }
}

impl fmt::Display for WatcherSupervisorStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "watcher host-fatal {:?}", self.kind)?;
        if let Some(repo_id) = self.repo_id {
            write!(formatter, " for repo {repo_id}")?;
        }
        write!(formatter, ": {}", self.primary)?;
        write_failures(formatter, &self.cleanup)
    }
}

impl std::error::Error for WatcherSupervisorStartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.failure
            .as_deref()
            .map(|failure| failure as &(dyn std::error::Error + 'static))
    }
}

#[derive(Debug)]
pub(crate) struct WatcherSupervisorShutdownError {
    pub(super) failures: Vec<WatcherFailure>,
}

impl fmt::Display for WatcherSupervisorShutdownError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("watcher supervisor shutdown failed")?;
        write_failures(formatter, &self.failures)
    }
}

impl std::error::Error for WatcherSupervisorShutdownError {}

fn write_failures(formatter: &mut fmt::Formatter<'_>, failures: &[WatcherFailure]) -> fmt::Result {
    for failure in failures {
        write!(formatter, "; cleanup failure: {failure}")?;
    }
    Ok(())
}

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
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) enum WatcherSupervisorStartError {
    DuplicateRepo(RepoId),
    Coordination {
        cleanup: Vec<WatcherFailure>,
    },
    Start {
        repo_id: RepoId,
        source: WatcherStartError,
        cleanup: Vec<WatcherFailure>,
    },
    FailedBeforeMounted {
        repo_id: RepoId,
        failure: WatcherFailure,
        cleanup: Vec<WatcherFailure>,
    },
}

impl fmt::Display for WatcherSupervisorStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateRepo(repo_id) => {
                write!(
                    formatter,
                    "duplicate watcher reservation for repo {repo_id}"
                )
            }
            Self::Coordination { cleanup } => {
                formatter.write_str("watcher supervisor coordination failed")?;
                write_failures(formatter, cleanup)
            }
            Self::Start {
                repo_id,
                source,
                cleanup,
            } => {
                write!(
                    formatter,
                    "watcher start failed for repo {repo_id}: {source}"
                )?;
                write_failures(formatter, cleanup)
            }
            Self::FailedBeforeMounted {
                repo_id,
                failure,
                cleanup,
            } => {
                write!(
                    formatter,
                    "watcher failed before mount handoff for repo {repo_id}: {failure}"
                )?;
                write_failures(formatter, cleanup)
            }
        }
    }
}

impl std::error::Error for WatcherSupervisorStartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Start { source, .. } => Some(source),
            Self::FailedBeforeMounted { failure, .. } => Some(failure),
            Self::DuplicateRepo(_) | Self::Coordination { .. } => None,
        }
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

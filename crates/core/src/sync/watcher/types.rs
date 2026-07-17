//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!
//! Public watcher runtime contracts. These types describe process-local
//! ingestion readiness; they are not durable repository health.

use crate::models::RepoId;
use crate::sync::SyncManager;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

use super::DEFAULT_DEBOUNCE;
use super::WatcherRefreshCallback;

/// Synchronous terminal-failure notification used by the host runtime to
/// close mutation admission at the same cut where the worker observes its
/// primary failure. Implementations must be non-blocking and must not panic.
pub type WatcherFailureCallback = Arc<dyn Fn(WatcherFailure) + Send + Sync + 'static>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatcherFailurePhase {
    Prepare,
    Attach,
    InitialScan,
    WorkerSpawn,
    Worker,
    Receive,
    Dispatch,
    Reconcile,
    Shutdown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatcherFailureKind {
    Configuration,
    Repository,
    Backend,
    Scan,
    Dispatch,
    Thread,
    Panic,
    StartupChurn,
    Coordination,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("watcher {phase:?}/{kind:?}: {primary}")]
pub struct WatcherFailure {
    pub phase: WatcherFailurePhase,
    pub kind: WatcherFailureKind,
    pub primary: String,
    pub cleanup: Vec<String>,
}

impl WatcherFailure {
    pub fn new(
        phase: WatcherFailurePhase,
        kind: WatcherFailureKind,
        primary: impl Into<String>,
    ) -> Self {
        Self {
            phase,
            kind,
            primary: primary.into(),
            cleanup: Vec::new(),
        }
    }

    pub(crate) fn with_cleanup(mut self, cleanup: impl Into<String>) -> Self {
        self.cleanup.push(cleanup.into());
        self
    }
}

#[derive(Debug, Error)]
#[error("{failure}")]
pub struct WatcherStartError {
    failure: WatcherFailure,
}

impl WatcherStartError {
    pub(crate) fn new(failure: WatcherFailure) -> Self {
        Self { failure }
    }

    pub fn failure(&self) -> &WatcherFailure {
        &self.failure
    }

    pub fn into_failure(self) -> WatcherFailure {
        self.failure
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepoWatcherWorkerState {
    Running,
    Failed(WatcherFailure),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoWatcherSnapshot {
    repo_id: RepoId,
    generation: u64,
    worker_state: RepoWatcherWorkerState,
}

impl RepoWatcherSnapshot {
    pub(crate) fn new(
        repo_id: RepoId,
        generation: u64,
        worker_state: RepoWatcherWorkerState,
    ) -> Self {
        Self {
            repo_id,
            generation,
            worker_state,
        }
    }

    pub fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn worker_state(&self) -> &RepoWatcherWorkerState {
        &self.worker_state
    }
}

pub struct RepoWatcherStart {
    pub(crate) sync: Arc<SyncManager>,
    pub(crate) repo_name: String,
    pub(crate) repo_id: RepoId,
    pub(crate) generation: u64,
    pub(crate) debounce: Duration,
    pub(crate) refresh: Option<WatcherRefreshCallback>,
    pub(crate) failure: Option<WatcherFailureCallback>,
}

impl RepoWatcherStart {
    pub fn new(
        sync: Arc<SyncManager>,
        repo_id: RepoId,
        repo_name: impl Into<String>,
        generation: u64,
    ) -> Self {
        Self {
            sync,
            repo_name: repo_name.into(),
            repo_id,
            generation,
            debounce: DEFAULT_DEBOUNCE,
            refresh: None,
            failure: None,
        }
    }

    pub fn resolve(
        sync: Arc<SyncManager>,
        repo_name: impl Into<String>,
        generation: u64,
    ) -> Result<Self, WatcherStartError> {
        let repo_name = repo_name.into();
        let info = sync
            .repo
            .get_repo_info_for(None, Some(&repo_name))
            .map_err(|error| {
                WatcherStartError::new(WatcherFailure::new(
                    WatcherFailurePhase::Prepare,
                    WatcherFailureKind::Repository,
                    error.to_string(),
                ))
            })?
            .ok_or_else(|| {
                WatcherStartError::new(WatcherFailure::new(
                    WatcherFailurePhase::Prepare,
                    WatcherFailureKind::Repository,
                    format!("repository missing: {repo_name}"),
                ))
            })?;
        Ok(Self::new(sync, info.uuid, repo_name, generation))
    }

    pub fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn with_debounce(mut self, debounce: Duration) -> Self {
        self.debounce = debounce;
        self
    }

    pub fn with_refresh(mut self, refresh: WatcherRefreshCallback) -> Self {
        self.refresh = Some(refresh);
        self
    }

    pub fn with_failure_callback(mut self, failure: WatcherFailureCallback) -> Self {
        self.failure = Some(failure);
        self
    }
}

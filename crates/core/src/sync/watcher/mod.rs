//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

pub(crate) mod backend;
pub use backend::{
    BackendHintBatch, BackendSignal, FsEventHint, FsEventHintKind, FsEventPath, ReconcileToken,
};
mod dispatch;
#[cfg(test)]
mod dispatch_burst_test;
#[cfg(test)]
mod dispatch_test;
#[cfg(test)]
mod dispatch_test_support;
mod filter;
mod handle;
mod refresh;
mod types;
mod worker;

use std::time::Duration;
use thiserror::Error;

pub use handle::RepoWatcherHandle;
pub use refresh::{WatcherRefresh, WatcherRefreshCallback, WatcherRefreshKind};
pub use types::{
    RepoWatcherSnapshot, RepoWatcherStart, RepoWatcherWorkerState, WatcherFailure,
    WatcherFailureKind, WatcherFailurePhase, WatcherStartError,
};

pub(crate) type WatcherCallback = WatcherRefreshCallback;
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Debug, Error)]
pub(crate) enum WatcherError {
    #[error("WatcherInitFailed: {0}")]
    WatcherInitFailed(String),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

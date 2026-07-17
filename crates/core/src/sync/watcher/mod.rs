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
mod startup;
mod types;
mod worker;

use std::time::Duration;
use thiserror::Error;

pub use handle::RepoWatcherHandle;
pub use refresh::{WatcherRefresh, WatcherRefreshCallback, WatcherRefreshKind};
pub use types::{
    RepoWatcherSnapshot, RepoWatcherStart, RepoWatcherWorkerState, WatcherFailure,
    WatcherFailureCallback, WatcherFailureKind, WatcherFailurePhase, WatcherStartError,
};

pub(crate) type WatcherCallback = WatcherRefreshCallback;
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(150);

pub(crate) fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        "watcher runtime panicked".to_owned()
    }
}

#[derive(Debug, Error)]
pub(crate) enum WatcherError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

//! plan_ref:
//!   - 04_storage.md §Watcher Architecture

use super::WatcherError;
use notify_debouncer_full::DebouncedEvent;
use std::path::Path;
use std::time::Duration;

pub(crate) mod notify_impl;

pub(crate) enum BackendBatch {
    Events(Vec<DebouncedEvent>),
    Rescan,
}

pub(crate) trait FsWatcherBackend: Send {
    fn recv(&self, timeout: Duration) -> Result<Option<BackendBatch>, WatcherError>;
    fn stop(&mut self) -> Result<(), WatcherError>;
}

pub(crate) fn desktop_backend(
    repo_root: &Path,
    debounce: Duration,
) -> Result<Box<dyn FsWatcherBackend>, WatcherError> {
    notify_impl::start(repo_root, debounce)
}

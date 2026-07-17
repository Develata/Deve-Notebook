//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::{WatcherError, WatcherFailure};
use std::path::Path;
use std::time::Duration;

mod capture;
pub(crate) mod notify_impl;
#[cfg(test)]
mod startup_test_support;
#[cfg(test)]
pub(crate) use startup_test_support::{StartupCaptureControl, startup_capture_backend};

pub const MAX_QUEUED_HINT_BATCHES: usize = 16;
pub const MAX_HINTS_PER_BATCH: usize = 256;
pub const MAX_HINT_PATH_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FsEventPath(String);

impl FsEventPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn new(path: String) -> Option<Self> {
        let has_drive_prefix = path
            .as_bytes()
            .get(..2)
            .is_some_and(|prefix| prefix[0].is_ascii_alphabetic() && prefix[1] == b':');
        if path.contains(['\0', '\\']) || path.starts_with('/') || has_drive_prefix {
            return None;
        }
        if !path.is_empty()
            && path
                .split('/')
                .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
        {
            return None;
        }
        Some(Self(path))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FsEventHint {
    kind: FsEventHintKind,
    paths: Vec<FsEventPath>,
}

impl FsEventHint {
    pub(crate) fn changed(path: FsEventPath) -> Self {
        Self {
            kind: FsEventHintKind::Changed,
            paths: vec![path],
        }
    }

    pub(crate) fn removed_file(path: FsEventPath) -> Self {
        Self {
            kind: FsEventHintKind::RemovedFile,
            paths: vec![path],
        }
    }

    pub(crate) fn removed_directory(path: FsEventPath) -> Self {
        Self {
            kind: FsEventHintKind::RemovedDirectory,
            paths: vec![path],
        }
    }

    pub(crate) fn removed_unknown(path: FsEventPath) -> Self {
        Self {
            kind: FsEventHintKind::RemovedUnknown,
            paths: vec![path],
        }
    }

    pub(crate) fn rename(old_path: FsEventPath, new_path: FsEventPath) -> Self {
        Self {
            kind: FsEventHintKind::Rename,
            paths: vec![old_path, new_path],
        }
    }

    pub(crate) fn kind(&self) -> FsEventHintKind {
        self.kind
    }

    pub(crate) fn paths(&self) -> &[FsEventPath] {
        &self.paths
    }

    fn path_payload_bytes(&self) -> usize {
        self.paths.iter().map(|path| path.0.len()).sum()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FsEventHintKind {
    Changed,
    RemovedFile,
    RemovedDirectory,
    RemovedUnknown,
    Rename,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReconcileToken(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StartupScanToken {
    state: u64,
    generation: u64,
}

impl StartupScanToken {
    pub(crate) fn new(state: u64, generation: u64) -> Self {
        Self { state, generation }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StartupHandoff {
    Running,
    Dirty,
}

pub struct BackendHintBatch {
    hints: Vec<FsEventHint>,
    _claim: capture::RunningClaim,
}

impl BackendHintBatch {
    pub fn hints(&self) -> &[FsEventHint] {
        &self.hints
    }

    pub fn len(&self) -> usize {
        self.hints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hints.is_empty()
    }

    fn new(hints: Vec<FsEventHint>, claim: capture::RunningClaim) -> Self {
        Self {
            hints,
            _claim: claim,
        }
    }
}

impl std::fmt::Debug for BackendHintBatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BackendHintBatch")
            .field("hints", &self.hints)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub enum BackendSignal {
    Hints(BackendHintBatch),
    Reconcile(ReconcileToken),
    Terminal(WatcherFailure),
}

pub(crate) trait FsWatcherBackend: Send {
    fn begin_startup_scan(&self) -> Result<StartupScanToken, WatcherFailure>;
    fn complete_startup_scan(
        &self,
        token: StartupScanToken,
    ) -> Result<StartupHandoff, WatcherFailure>;
    fn recv(&self, timeout: Duration) -> Result<Option<BackendSignal>, WatcherError>;
    fn complete_reconcile(&self, token: ReconcileToken) -> bool;
    fn stop(&mut self) -> Result<(), WatcherError>;
}

pub(crate) fn desktop_backend(
    repo_root: &Path,
    debounce: Duration,
    generation: u64,
) -> Result<Box<dyn FsWatcherBackend>, WatcherFailure> {
    notify_impl::start(repo_root, debounce, generation)
}

#[cfg(test)]
mod domain_tests {
    use super::FsEventPath;

    #[test]
    fn fs_event_path_accepts_only_normalized_repo_relative_utf8() {
        assert!(FsEventPath::new(String::new()).is_some());
        assert!(FsEventPath::new("notes/a.md".into()).is_some());
        for invalid in [
            "/notes/a.md",
            "../a.md",
            "notes/./a.md",
            "notes//a.md",
            "notes\\a.md",
            "C:/outside.md",
            "C:outside.md",
        ] {
            assert!(
                FsEventPath::new(invalid.into()).is_none(),
                "invalid path must fail closed: {invalid}"
            );
        }
    }
}

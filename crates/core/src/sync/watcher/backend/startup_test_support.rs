//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::capture::{CaptureInput, CaptureReceiver, CaptureSender, bounded_capture};
use super::{BackendSignal, FsEventHint, FsEventPath, FsWatcherBackend, ReconcileToken};
use crate::sync::watcher::{WatcherError, WatcherFailure};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

pub(crate) struct StartupCaptureControl {
    sender: CaptureSender,
}

impl StartupCaptureControl {
    pub(crate) fn submit_changed(&self, repo_path: &str) {
        let path = FsEventPath::new(repo_path.to_string()).expect("valid test repo path");
        self.sender
            .submit(CaptureInput::Hints(vec![FsEventHint::changed(path)]));
    }
}

struct StartupCaptureBackend {
    receiver: CaptureReceiver,
    stopped: Arc<AtomicUsize>,
}

impl FsWatcherBackend for StartupCaptureBackend {
    fn begin_startup_scan(&self) -> Result<super::StartupScanToken, WatcherFailure> {
        self.receiver.begin_startup_scan()
    }

    fn complete_startup_scan(
        &self,
        token: super::StartupScanToken,
    ) -> Result<super::StartupHandoff, WatcherFailure> {
        self.receiver.complete_startup_scan(token)
    }

    fn recv(&self, timeout: Duration) -> Result<Option<BackendSignal>, WatcherError> {
        self.receiver.recv(timeout)
    }

    fn complete_reconcile(&self, token: ReconcileToken) -> bool {
        self.receiver.complete_reconcile(token)
    }

    fn stop(&mut self) -> Result<(), WatcherError> {
        self.stopped.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn discard_pending_hints(&self) {
        self.receiver.discard_pending_hints();
    }
}

pub(crate) fn startup_capture_backend(
    generation: u64,
    stopped: Arc<AtomicUsize>,
) -> (StartupCaptureControl, Box<dyn FsWatcherBackend>) {
    let (sender, receiver) = bounded_capture(generation);
    (
        StartupCaptureControl { sender },
        Box::new(StartupCaptureBackend { receiver, stopped }),
    )
}

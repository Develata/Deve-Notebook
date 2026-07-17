//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::*;
use crate::sync::watcher::WatcherError;
use crate::sync::watcher::backend::{
    BackendSignal, ReconcileToken, StartupCaptureControl, StartupHandoff, StartupScanToken,
    startup_capture_backend,
};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

struct ScriptedBackend {
    generation: u64,
    script: Arc<Mutex<Script>>,
    stopped: Arc<AtomicUsize>,
    stop_fails: bool,
}

struct Script {
    begins: u64,
    handoffs: VecDeque<Result<StartupHandoff, WatcherFailure>>,
}

impl ScriptedBackend {
    fn new(
        generation: u64,
        handoffs: Vec<Result<StartupHandoff, WatcherFailure>>,
        stopped: Arc<AtomicUsize>,
        stop_fails: bool,
    ) -> Self {
        Self {
            generation,
            script: Arc::new(Mutex::new(Script {
                begins: 0,
                handoffs: handoffs.into(),
            })),
            stopped,
            stop_fails,
        }
    }
}

impl FsWatcherBackend for ScriptedBackend {
    fn begin_startup_scan(&self) -> Result<StartupScanToken, WatcherFailure> {
        let mut script = self.script.lock().expect("script lock");
        script.begins += 1;
        Ok(StartupScanToken::new(script.begins, self.generation))
    }

    fn complete_startup_scan(
        &self,
        _token: StartupScanToken,
    ) -> Result<StartupHandoff, WatcherFailure> {
        self.script
            .lock()
            .expect("script lock")
            .handoffs
            .pop_front()
            .unwrap_or(Ok(StartupHandoff::Running))
    }

    fn recv(&self, timeout: Duration) -> Result<Option<BackendSignal>, WatcherError> {
        std::thread::sleep(timeout.min(Duration::from_millis(1)));
        Ok(None)
    }

    fn complete_reconcile(&self, _token: ReconcileToken) -> bool {
        false
    }

    fn stop(&mut self) -> Result<(), WatcherError> {
        self.stopped.fetch_add(1, Ordering::SeqCst);
        if self.stop_fails {
            Err(anyhow::anyhow!("injected startup stop cleanup").into())
        } else {
            Ok(())
        }
    }
}

fn fixture() -> (
    tempfile::TempDir,
    Arc<SyncManager>,
    String,
    RepoId,
    std::path::PathBuf,
) {
    let (dir, _repo, sync, repo_name, repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    (dir, sync, repo_name, repo_id, repo_root)
}

fn stop_started(started: StartedWatcher) {
    started.stop_tx.send(()).expect("stop worker");
    started
        .join
        .join()
        .expect("join worker")
        .expect("worker shutdown");
}

#[test]
fn watcher_capture_first_startup_covers_attach_to_first_scan_window() {
    let (_dir, sync, repo_name, repo_id, _repo_root) = fixture();
    let stopped = Arc::new(AtomicUsize::new(0));
    let backend_stopped = stopped.clone();
    let scans = Arc::new(AtomicUsize::new(0));
    let observed_scans = scans.clone();
    let capture = Arc::new(Mutex::new(None::<StartupCaptureControl>));
    let attached_capture = capture.clone();
    let scan_capture = capture.clone();
    let scanned_sync = sync.clone();
    let scanned_repo_name = repo_name.clone();

    let started = start_with(
        RepoWatcherStart::new(sync, repo_id, repo_name, 21),
        move |_, _, generation| {
            let (control, backend) = startup_capture_backend(generation, backend_stopped);
            *attached_capture.lock().expect("capture control lock") = Some(control);
            Ok(backend)
        },
        move |sync, repo_name, repo_root| {
            crate::sync::scan::scan_local_repo_at_root(
                &sync.repo, &sync.vfs, repo_name, repo_root,
            )?;
            if observed_scans.fetch_add(1, Ordering::SeqCst) == 0 {
                let during_scan = repo_root.join("notes").join("during-startup.md");
                std::fs::create_dir_all(during_scan.parent().expect("startup test parent"))?;
                std::fs::write(during_scan, "captured during startup")?;
                scan_capture
                    .lock()
                    .expect("capture control lock")
                    .as_ref()
                    .expect("backend attached before scan")
                    .submit_changed("notes/during-startup.md");
            }
            Ok(())
        },
    )
    .expect("second clean pass starts watcher");

    assert_eq!(scans.load(Ordering::SeqCst), 2);
    let pending = scanned_sync
        .repo
        .list_pending_fs_in_local_repo(&scanned_repo_name)
        .expect("startup pending view");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/during-startup.md");
    stop_started(started);
    assert_eq!(stopped.load(Ordering::SeqCst), 1);
}

#[test]
fn watcher_capture_first_startup_returns_churn_after_exactly_three_dirty_passes() {
    let (_dir, sync, repo_name, repo_id, _repo_root) = fixture();
    let stopped = Arc::new(AtomicUsize::new(0));
    let backend_stopped = stopped.clone();
    let scans = Arc::new(AtomicUsize::new(0));
    let observed_scans = scans.clone();

    let error = match start_with(
        RepoWatcherStart::new(sync, repo_id, repo_name, 22),
        move |_, _, generation| {
            Ok(Box::new(ScriptedBackend::new(
                generation,
                vec![
                    Ok(StartupHandoff::Dirty),
                    Ok(StartupHandoff::Dirty),
                    Ok(StartupHandoff::Dirty),
                ],
                backend_stopped,
                false,
            )))
        },
        move |_, _, _| {
            observed_scans.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    ) {
        Ok(_) => panic!("continuous startup churn must not publish a handle"),
        Err(error) => error,
    };

    assert_eq!(error.failure().kind, WatcherFailureKind::StartupChurn);
    assert_eq!(scans.load(Ordering::SeqCst), 3);
    assert_eq!(stopped.load(Ordering::SeqCst), 1);
}

#[test]
fn watcher_capture_first_startup_scan_failure_preserves_stop_cleanup() {
    let (_dir, sync, repo_name, repo_id, _repo_root) = fixture();
    let stopped = Arc::new(AtomicUsize::new(0));
    let backend_stopped = stopped.clone();

    let error = match start_with(
        RepoWatcherStart::new(sync, repo_id, repo_name, 23),
        move |_, _, generation| {
            Ok(Box::new(ScriptedBackend::new(
                generation,
                vec![Ok(StartupHandoff::Running)],
                backend_stopped,
                true,
            )))
        },
        |_, _, _| Err(anyhow::anyhow!("injected startup scan primary")),
    ) {
        Ok(_) => panic!("failed scan must not publish a handle"),
        Err(error) => error,
    };

    assert_eq!(error.failure().kind, WatcherFailureKind::Scan);
    assert!(error.failure().primary.contains("startup scan primary"));
    assert!(
        error
            .failure()
            .cleanup
            .iter()
            .any(|cleanup| cleanup.contains("startup stop cleanup"))
    );
    assert_eq!(stopped.load(Ordering::SeqCst), 1);
}

#[test]
fn watcher_capture_first_startup_terminal_handoff_preserves_first_failure() {
    let (_dir, sync, repo_name, repo_id, _repo_root) = fixture();
    let stopped = Arc::new(AtomicUsize::new(0));
    let backend_stopped = stopped.clone();
    let terminal = WatcherFailure::new(
        WatcherFailurePhase::Receive,
        WatcherFailureKind::Panic,
        "injected callback panic",
    );
    let expected = terminal.clone();

    let error = match start_with(
        RepoWatcherStart::new(sync, repo_id, repo_name, 24),
        move |_, _, generation| {
            Ok(Box::new(ScriptedBackend::new(
                generation,
                vec![Err(terminal)],
                backend_stopped,
                false,
            )))
        },
        |_, _, _| Ok(()),
    ) {
        Ok(_) => panic!("terminal startup failure must not publish a handle"),
        Err(error) => error,
    };

    assert_eq!(error.failure(), &expected);
    assert_ne!(error.failure().kind, WatcherFailureKind::StartupChurn);
    assert_eq!(stopped.load(Ordering::SeqCst), 1);
}

#[test]
fn watcher_capture_first_startup_prepare_failure_never_attaches_backend() {
    let (_dir, sync, repo_name, repo_id, _repo_root) = fixture();
    let attaches = Arc::new(AtomicUsize::new(0));
    let observed_attaches = attaches.clone();

    let error = match start_with(
        RepoWatcherStart::new(sync, repo_id, repo_name, 0),
        move |_, _, _| {
            observed_attaches.fetch_add(1, Ordering::SeqCst);
            Err(WatcherFailure::new(
                WatcherFailurePhase::Attach,
                WatcherFailureKind::Backend,
                "must not attach",
            ))
        },
        |_, _, _| Ok(()),
    ) {
        Ok(_) => panic!("zero generation must fail during prepare"),
        Err(error) => error,
    };

    assert_eq!(error.failure().phase, WatcherFailurePhase::Prepare);
    assert_eq!(attaches.load(Ordering::SeqCst), 0);
}

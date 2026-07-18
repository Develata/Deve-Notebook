//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::*;
use crate::sync::watcher::backend::{ReconcileToken, StartupHandoff, StartupScanToken};
use std::sync::atomic::{AtomicBool, Ordering};

struct PanickingBackend {
    stopped: Arc<AtomicBool>,
}

struct FailingBackend;

struct StopPanickingBackend;

struct BlockingCleanupBackend {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

impl FsWatcherBackend for FailingBackend {
    fn begin_startup_scan(&self) -> Result<StartupScanToken, WatcherFailure> {
        unreachable!("worker-only backend does not participate in startup")
    }

    fn complete_startup_scan(
        &self,
        _token: StartupScanToken,
    ) -> Result<StartupHandoff, WatcherFailure> {
        unreachable!("worker-only backend does not participate in startup")
    }

    fn recv(
        &self,
        _timeout: Duration,
    ) -> Result<Option<BackendSignal>, super::super::WatcherError> {
        Err(anyhow::anyhow!("injected receive failure").into())
    }

    fn complete_reconcile(&self, _token: ReconcileToken) -> bool {
        false
    }

    fn stop(&mut self) -> Result<(), super::super::WatcherError> {
        Err(anyhow::anyhow!("injected cleanup failure").into())
    }

    fn discard_pending_hints(&self) {}
}

impl FsWatcherBackend for StopPanickingBackend {
    fn begin_startup_scan(&self) -> Result<StartupScanToken, WatcherFailure> {
        unreachable!("worker-only backend does not participate in startup")
    }

    fn complete_startup_scan(
        &self,
        _token: StartupScanToken,
    ) -> Result<StartupHandoff, WatcherFailure> {
        unreachable!("worker-only backend does not participate in startup")
    }

    fn recv(
        &self,
        _timeout: Duration,
    ) -> Result<Option<BackendSignal>, super::super::WatcherError> {
        Ok(None)
    }

    fn complete_reconcile(&self, _token: ReconcileToken) -> bool {
        false
    }

    fn stop(&mut self) -> Result<(), super::super::WatcherError> {
        panic!("injected cleanup panic")
    }

    fn discard_pending_hints(&self) {}
}

impl FsWatcherBackend for BlockingCleanupBackend {
    fn begin_startup_scan(&self) -> Result<StartupScanToken, WatcherFailure> {
        unreachable!("worker-only backend does not participate in startup")
    }

    fn complete_startup_scan(
        &self,
        _token: StartupScanToken,
    ) -> Result<StartupHandoff, WatcherFailure> {
        unreachable!("worker-only backend does not participate in startup")
    }

    fn recv(
        &self,
        _timeout: Duration,
    ) -> Result<Option<BackendSignal>, super::super::WatcherError> {
        Err(anyhow::anyhow!("terminal receive failure").into())
    }

    fn complete_reconcile(&self, _token: ReconcileToken) -> bool {
        false
    }

    fn stop(&mut self) -> Result<(), super::super::WatcherError> {
        self.entered.send(()).expect("announce cleanup");
        self.release.recv().expect("release cleanup");
        Ok(())
    }

    fn discard_pending_hints(&self) {}
}

impl FsWatcherBackend for PanickingBackend {
    fn begin_startup_scan(&self) -> Result<StartupScanToken, WatcherFailure> {
        unreachable!("worker-only backend does not participate in startup")
    }

    fn complete_startup_scan(
        &self,
        _token: StartupScanToken,
    ) -> Result<StartupHandoff, WatcherFailure> {
        unreachable!("worker-only backend does not participate in startup")
    }

    fn recv(
        &self,
        _timeout: Duration,
    ) -> Result<Option<BackendSignal>, super::super::WatcherError> {
        panic!("injected watcher panic")
    }

    fn complete_reconcile(&self, _token: ReconcileToken) -> bool {
        false
    }

    fn stop(&mut self) -> Result<(), super::super::WatcherError> {
        self.stopped.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn discard_pending_hints(&self) {}
}

#[test]
fn worker_panic_becomes_typed_failure_and_stops_backend() {
    let (_dir, _repo, sync, repo_name, repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    let stopped = Arc::new(AtomicBool::new(false));
    let state = Arc::new(RwLock::new(WorkerStateSlot::running(1)));
    let (_command_tx, command_rx) = mpsc::channel();

    let failure = run(WorkerInput {
        failure: None,
        sync,
        repo_name,
        repo_id,
        generation: 1,
        repo_root,
        backend: Box::new(PanickingBackend {
            stopped: stopped.clone(),
        }),
        command_rx,
        refresh: None,
        state: state.clone(),
    })
    .expect_err("panic must become terminal failure");

    assert_eq!(failure.kind, WatcherFailureKind::Panic);
    assert!(failure.primary.contains("injected watcher panic"));
    assert!(stopped.load(Ordering::SeqCst));
    assert!(matches!(
        state.read().expect("state").snapshot(1),
        Some(RepoWatcherWorkerState::Failed(observed)) if observed == failure
    ));
}

#[test]
fn failure_callback_panic_cannot_skip_state_cut_or_backend_cleanup() {
    let (_dir, _repo, sync, repo_name, repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    let stopped = Arc::new(AtomicBool::new(false));
    let state = Arc::new(RwLock::new(WorkerStateSlot::running(1)));
    let (_command_tx, command_rx) = mpsc::channel();

    let failure = run(WorkerInput {
        failure: Some(Arc::new(|_| panic!("injected callback panic"))),
        sync,
        repo_name,
        repo_id,
        generation: 1,
        repo_root,
        backend: Box::new(PanickingBackend {
            stopped: stopped.clone(),
        }),
        command_rx,
        refresh: None,
        state: state.clone(),
    })
    .expect_err("worker and callback panic must become terminal failure");

    assert_eq!(failure.kind, WatcherFailureKind::Panic);
    assert!(failure.primary.contains("injected watcher panic"));
    assert!(
        failure
            .cleanup
            .iter()
            .any(|cleanup| cleanup.contains("injected callback panic"))
    );
    assert!(stopped.load(Ordering::SeqCst));
    assert!(matches!(
        state.read().expect("state").snapshot(1),
        Some(RepoWatcherWorkerState::Failed(observed)) if observed == failure
    ));
}

#[test]
fn consumer_failure_preserves_primary_and_cleanup() {
    let (_dir, _repo, sync, repo_name, repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    let state = Arc::new(RwLock::new(WorkerStateSlot::running(1)));
    let (_command_tx, command_rx) = mpsc::channel();

    let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
    let callback_observed = observed.clone();
    let failure = run(WorkerInput {
        failure: Some(Arc::new(move |failure| {
            callback_observed
                .lock()
                .expect("failure callback observations")
                .push(failure);
        })),
        sync,
        repo_name,
        repo_id,
        generation: 1,
        repo_root,
        backend: Box::new(FailingBackend),
        command_rx,
        refresh: None,
        state,
    })
    .expect_err("receive and cleanup failure must escape");

    assert_eq!(failure.phase, WatcherFailurePhase::Receive);
    assert_eq!(failure.kind, WatcherFailureKind::Backend);
    assert!(failure.primary.contains("injected receive failure"));
    assert_eq!(failure.cleanup.len(), 1);
    assert!(failure.cleanup[0].contains("injected cleanup failure"));
    let observed = observed.lock().expect("failure callback observations");
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].phase, WatcherFailurePhase::Receive);
    assert!(observed[0].cleanup.is_empty());
}

#[test]
fn full_rescan_emits_repo_scoped_refresh() {
    let (_dir, _repo, sync, repo_name, repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    let refreshes = Arc::new(std::sync::Mutex::new(Vec::new()));
    let observed = refreshes.clone();
    let callback: WatcherRefreshCallback = Arc::new(move |refresh| {
        observed.lock().expect("refresh lock").push(refresh);
    });

    rescan_and_notify(&sync, &repo_name, repo_id, &repo_root, Some(&callback))
        .expect("full watcher rescan");

    let refreshes = refreshes.lock().expect("refresh lock");
    assert_eq!(refreshes.len(), 1);
    assert_eq!(refreshes[0].repo_id(), repo_id);
    assert_eq!(refreshes[0].path(), "");
    assert_eq!(
        refreshes[0].kind(),
        super::super::WatcherRefreshKind::DirectoryChanged
    );
}

#[test]
fn cleanup_panic_becomes_typed_shutdown_failure() {
    let (_dir, _repo, sync, repo_name, repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    let state = Arc::new(RwLock::new(WorkerStateSlot::running(1)));
    let (command_tx, command_rx) = mpsc::channel();
    command_tx
        .send(WorkerCommand::Shutdown)
        .expect("request stop");

    let failure = run(WorkerInput {
        failure: None,
        sync,
        repo_name,
        repo_id,
        generation: 1,
        repo_root,
        backend: Box::new(StopPanickingBackend),
        command_rx,
        refresh: None,
        state,
    })
    .expect_err("cleanup panic must become typed failure");

    assert_eq!(failure.phase, WatcherFailurePhase::Shutdown);
    assert_eq!(failure.kind, WatcherFailureKind::Panic);
    assert!(failure.primary.contains("injected cleanup panic"));
}

#[test]
fn terminal_failure_is_visible_before_cleanup_completes() {
    let (_dir, _repo, sync, repo_name, repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    let state = Arc::new(RwLock::new(WorkerStateSlot::running(1)));
    let observed = state.clone();
    let (_command_tx, command_rx) = mpsc::channel();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (failure_tx, failure_rx) = mpsc::channel();

    let join = std::thread::spawn(move || {
        run(WorkerInput {
            failure: Some(Arc::new(move |failure| {
                failure_tx.send(failure).expect("publish failure cut");
            })),
            sync,
            repo_name,
            repo_id,
            generation: 1,
            repo_root,
            backend: Box::new(BlockingCleanupBackend {
                entered: entered_tx,
                release: release_rx,
            }),
            command_rx,
            refresh: None,
            state,
        })
    });
    entered_rx.recv().expect("cleanup entered");

    let cut = failure_rx.recv().expect("failure cut before cleanup");
    assert_eq!(cut.phase, WatcherFailurePhase::Receive);

    assert!(matches!(
        observed.read().expect("state").snapshot(1),
        Some(RepoWatcherWorkerState::Failed(failure))
            if failure.phase == WatcherFailurePhase::Receive
    ));

    release_tx.send(()).expect("release cleanup");
    join.join()
        .expect("worker join")
        .expect_err("receive failure remains terminal");
}

#[test]
fn watcher_capture_first_startup_rejects_stale_generation_completion() {
    let state = RwLock::new(WorkerStateSlot::running(2));
    let stale_failure = WatcherFailure::new(
        WatcherFailurePhase::Receive,
        WatcherFailureKind::Backend,
        "stale generation failure",
    );

    assert!(!replace_state(
        &state,
        1,
        RepoWatcherWorkerState::Failed(stale_failure)
    ));
    assert_eq!(
        state.read().expect("state").snapshot(2),
        Some(RepoWatcherWorkerState::Running)
    );

    let current_failure = WatcherFailure::new(
        WatcherFailurePhase::Receive,
        WatcherFailureKind::Backend,
        "current generation failure",
    );
    assert!(replace_state(
        &state,
        2,
        RepoWatcherWorkerState::Failed(current_failure.clone())
    ));
    assert_eq!(
        state.read().expect("state").snapshot(2),
        Some(RepoWatcherWorkerState::Failed(current_failure))
    );
}

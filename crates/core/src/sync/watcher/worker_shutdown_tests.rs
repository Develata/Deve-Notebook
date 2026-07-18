//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::*;
use crate::sync::watcher::backend::{ReconcileToken, StartupHandoff, StartupScanToken};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Clone, Copy)]
enum ReceiveMode {
    MustNotPoll,
    Fail,
}

struct ShutdownProbeBackend {
    events: Arc<Mutex<Vec<&'static str>>>,
    receive: ReceiveMode,
    stop_fails: bool,
}

struct OneReconcileBackend {
    emitted: AtomicBool,
    stopped: Arc<AtomicBool>,
}

impl FsWatcherBackend for ShutdownProbeBackend {
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
        match self.receive {
            ReceiveMode::MustNotPoll => {
                panic!("shutdown request must prevent another backend poll")
            }
            ReceiveMode::Fail => Err(anyhow::anyhow!("injected receive primary").into()),
        }
    }

    fn complete_reconcile(&self, _token: ReconcileToken) -> bool {
        false
    }

    fn stop(&mut self) -> Result<(), super::super::WatcherError> {
        self.events.lock().expect("event log").push("stop");
        if self.stop_fails {
            Err(anyhow::anyhow!("injected stop cleanup").into())
        } else {
            Ok(())
        }
    }

    fn discard_pending_hints(&self) {
        self.events.lock().expect("event log").push("discard");
    }
}

impl FsWatcherBackend for OneReconcileBackend {
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
        if self.emitted.swap(true, Ordering::SeqCst) {
            Ok(None)
        } else {
            Ok(Some(BackendSignal::Reconcile(ReconcileToken::for_test(1))))
        }
    }

    fn complete_reconcile(&self, _token: ReconcileToken) -> bool {
        true
    }

    fn stop(&mut self) -> Result<(), super::super::WatcherError> {
        self.stopped.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn discard_pending_hints(&self) {}
}

#[test]
fn watcher_final_state_shutdown_stops_discards_reconciles_then_refreshes() {
    let (_dir, _repo, sync, repo_name, repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    let final_path = repo_root.join("notes/final-state.md");
    std::fs::create_dir_all(final_path.parent().expect("final-state parent"))
        .expect("create final-state parent");
    std::fs::write(&final_path, "final state").expect("write final-state file");

    let events = Arc::new(Mutex::new(Vec::new()));
    let refresh_count = Arc::new(AtomicUsize::new(0));
    let observed_events = events.clone();
    let observed_refresh_count = refresh_count.clone();
    let refresh: WatcherRefreshCallback = Arc::new(move |_| {
        observed_events.lock().expect("event log").push("refresh");
        observed_refresh_count.fetch_add(1, Ordering::SeqCst);
    });
    let state = Arc::new(RwLock::new(WorkerStateSlot::running(1)));
    let (command_tx, command_rx) = mpsc::channel();
    command_tx
        .send(WorkerCommand::Shutdown)
        .expect("request shutdown");

    run(WorkerInput {
        failure: None,
        sync: sync.clone(),
        repo_name: repo_name.clone(),
        repo_id,
        generation: 1,
        repo_root,
        backend: Box::new(ShutdownProbeBackend {
            events: events.clone(),
            receive: ReceiveMode::MustNotPoll,
            stop_fails: false,
        }),
        command_rx,
        refresh: Some(refresh),
        state,
    })
    .expect("final-state shutdown");

    assert_eq!(
        events.lock().expect("event log").as_slice(),
        ["stop", "discard", "refresh"]
    );
    assert_eq!(refresh_count.load(Ordering::SeqCst), 1);
    let pending = sync
        .repo
        .list_pending_fs_in_local_repo(&repo_name)
        .expect("final-state pending view");
    assert!(
        pending
            .iter()
            .any(|entry| entry.path == "notes/final-state.md")
    );
}

#[test]
fn watcher_final_state_shutdown_waits_for_inflight_refresh_before_quiesced_cut() {
    let (_dir, _repo, sync, repo_name, repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    let state = Arc::new(RwLock::new(WorkerStateSlot::running(1)));
    let stopped = Arc::new(AtomicBool::new(false));
    let backend_stopped = stopped.clone();
    let refresh_count = Arc::new(AtomicUsize::new(0));
    let observed_refresh_count = refresh_count.clone();
    let (refresh_entered_tx, refresh_entered_rx) = mpsc::channel();
    let (release_refresh_tx, release_refresh_rx) = mpsc::channel();
    let release_refresh_rx = Arc::new(Mutex::new(release_refresh_rx));
    let callback_release = release_refresh_rx.clone();
    let refresh: WatcherRefreshCallback = Arc::new(move |_| {
        if observed_refresh_count.fetch_add(1, Ordering::SeqCst) == 0 {
            refresh_entered_tx
                .send(())
                .expect("announce in-flight refresh");
            callback_release
                .lock()
                .expect("refresh release lock")
                .recv()
                .expect("release in-flight refresh");
        }
    });
    let (command_tx, command_rx) = mpsc::channel();
    let (worker_done_tx, worker_done_rx) = mpsc::channel();

    let join = std::thread::spawn(move || {
        let result = run(WorkerInput {
            failure: None,
            sync,
            repo_name,
            repo_id,
            generation: 1,
            repo_root,
            backend: Box::new(OneReconcileBackend {
                emitted: AtomicBool::new(false),
                stopped: backend_stopped,
            }),
            command_rx,
            refresh: Some(refresh),
            state,
        });
        worker_done_tx.send(()).expect("announce worker exit");
        result
    });
    refresh_entered_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("first reconcile refresh entered");

    command_tx
        .send(WorkerCommand::Shutdown)
        .expect("request shutdown during refresh");
    assert!(
        worker_done_rx
            .recv_timeout(Duration::from_millis(50))
            .is_err(),
        "shutdown must wait for the in-flight refresh boundary"
    );
    assert!(
        !stopped.load(Ordering::SeqCst),
        "the single worker cannot enter producer-stop until dispatch returns"
    );

    release_refresh_tx.send(()).expect("release refresh");
    join.join()
        .expect("worker join")
        .expect("final-state shutdown");
    assert!(stopped.load(Ordering::SeqCst));
    assert_eq!(
        refresh_count.load(Ordering::SeqCst),
        2,
        "one running reconcile refresh plus exactly one final refresh"
    );
}

#[test]
fn watcher_final_state_shutdown_reports_final_scan_failure_after_cleanup() {
    let (_dir, _repo, sync, _repo_name, repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    let events = Arc::new(Mutex::new(Vec::new()));
    let state = Arc::new(RwLock::new(WorkerStateSlot::running(1)));
    let (command_tx, command_rx) = mpsc::channel();
    command_tx
        .send(WorkerCommand::Shutdown)
        .expect("request shutdown");

    let failure = run(WorkerInput {
        failure: None,
        sync,
        repo_name: "missing-repo".into(),
        repo_id,
        generation: 1,
        repo_root,
        backend: Box::new(ShutdownProbeBackend {
            events: events.clone(),
            receive: ReceiveMode::MustNotPoll,
            stop_fails: false,
        }),
        command_rx,
        refresh: None,
        state,
    })
    .expect_err("final scan failure must escape shutdown");

    assert_eq!(failure.phase, WatcherFailurePhase::Reconcile);
    assert_eq!(failure.kind, WatcherFailureKind::Scan);
    assert_eq!(
        events.lock().expect("event log").as_slice(),
        ["stop", "discard"]
    );
}

#[test]
fn watcher_final_state_shutdown_revalidates_exact_repo_identity() {
    let (_dir, _repo, sync, repo_name, _repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    let wrong_repo_id = RepoId::new_v4();
    let refresh_count = Arc::new(AtomicUsize::new(0));
    let observed_refresh_count = refresh_count.clone();
    let refresh: WatcherRefreshCallback = Arc::new(move |_| {
        observed_refresh_count.fetch_add(1, Ordering::SeqCst);
    });
    let events = Arc::new(Mutex::new(Vec::new()));
    let state = Arc::new(RwLock::new(WorkerStateSlot::running(1)));
    let (command_tx, command_rx) = mpsc::channel();
    command_tx
        .send(WorkerCommand::Shutdown)
        .expect("request shutdown");

    let failure = run(WorkerInput {
        failure: None,
        sync,
        repo_name,
        repo_id: wrong_repo_id,
        generation: 1,
        repo_root,
        backend: Box::new(ShutdownProbeBackend {
            events,
            receive: ReceiveMode::MustNotPoll,
            stop_fails: false,
        }),
        command_rx,
        refresh: Some(refresh),
        state,
    })
    .expect_err("repo identity drift must fail final reconcile");

    assert_eq!(failure.phase, WatcherFailurePhase::Reconcile);
    assert_eq!(failure.kind, WatcherFailureKind::Scan);
    assert!(failure.primary.contains("repository identity changed"));
    assert_eq!(refresh_count.load(Ordering::SeqCst), 0);
}

#[test]
fn watcher_final_state_shutdown_worker_failure_preserves_primary_and_appends_cleanup() {
    let (_dir, _repo, sync, _repo_name, repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    let events = Arc::new(Mutex::new(Vec::new()));
    let state = Arc::new(RwLock::new(WorkerStateSlot::running(1)));
    let (_command_tx, command_rx) = mpsc::channel();

    let failure = run(WorkerInput {
        failure: None,
        sync,
        repo_name: "missing-repo".into(),
        repo_id,
        generation: 1,
        repo_root,
        backend: Box::new(ShutdownProbeBackend {
            events: events.clone(),
            receive: ReceiveMode::Fail,
            stop_fails: true,
        }),
        command_rx,
        refresh: None,
        state,
    })
    .expect_err("worker primary must survive cleanup failures");

    assert_eq!(failure.phase, WatcherFailurePhase::Receive);
    assert_eq!(failure.kind, WatcherFailureKind::Backend);
    assert!(failure.primary.contains("injected receive primary"));
    assert!(
        failure
            .cleanup
            .iter()
            .any(|cleanup| cleanup.contains("injected stop cleanup"))
    );
    assert!(
        failure
            .cleanup
            .iter()
            .any(|cleanup| cleanup.contains("missing-repo"))
    );
    assert_eq!(
        events.lock().expect("event log").as_slice(),
        ["stop", "discard"]
    );
}

#[test]
fn watcher_final_state_shutdown_worker_failure_reconciles_and_refreshes_once() {
    let (_dir, _repo, sync, repo_name, repo_id, repo_root) =
        super::super::dispatch_test_support::new_sync().expect("watcher fixture");
    let final_path = repo_root.join("notes/failure-final.md");
    std::fs::create_dir_all(final_path.parent().expect("failure-final parent"))
        .expect("create failure-final parent");
    std::fs::write(&final_path, "failure final state").expect("write failure-final file");

    let events = Arc::new(Mutex::new(Vec::new()));
    let refresh_count = Arc::new(AtomicUsize::new(0));
    let observed_refresh_count = refresh_count.clone();
    let refresh: WatcherRefreshCallback = Arc::new(move |_| {
        observed_refresh_count.fetch_add(1, Ordering::SeqCst);
    });
    let state = Arc::new(RwLock::new(WorkerStateSlot::running(1)));
    let (_command_tx, command_rx) = mpsc::channel();

    let failure = run(WorkerInput {
        failure: None,
        sync: sync.clone(),
        repo_name: repo_name.clone(),
        repo_id,
        generation: 1,
        repo_root,
        backend: Box::new(ShutdownProbeBackend {
            events,
            receive: ReceiveMode::Fail,
            stop_fails: false,
        }),
        command_rx,
        refresh: Some(refresh),
        state,
    })
    .expect_err("receive failure remains primary");

    assert_eq!(failure.phase, WatcherFailurePhase::Receive);
    assert!(failure.cleanup.is_empty());
    assert_eq!(refresh_count.load(Ordering::SeqCst), 1);
    let pending = sync
        .repo
        .list_pending_fs_in_local_repo(&repo_name)
        .expect("failure final pending view");
    assert!(
        pending
            .iter()
            .any(|entry| entry.path == "notes/failure-final.md")
    );
}

//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::backend::{BackendSignal, FsWatcherBackend};
use super::{
    RepoWatcherWorkerState, WatcherFailure, WatcherFailureKind, WatcherFailurePhase,
    WatcherRefresh, WatcherRefreshCallback, dispatch,
};
use crate::models::RepoId;
use crate::sync::SyncManager;
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::{Arc, RwLock, mpsc};
use std::time::Duration;

pub(crate) struct WorkerInput {
    pub sync: Arc<SyncManager>,
    pub repo_name: String,
    pub repo_id: RepoId,
    pub repo_root: PathBuf,
    pub backend: Box<dyn FsWatcherBackend>,
    pub stop_rx: mpsc::Receiver<()>,
    pub refresh: Option<WatcherRefreshCallback>,
    pub state: Arc<RwLock<RepoWatcherWorkerState>>,
}

pub(crate) fn run(input: WorkerInput) -> Result<(), WatcherFailure> {
    let WorkerInput {
        sync,
        repo_name,
        repo_id,
        repo_root,
        mut backend,
        stop_rx,
        refresh,
        state,
    } = input;
    let primary = catch_unwind(AssertUnwindSafe(|| {
        consume_loop(
            sync,
            &repo_name,
            repo_id,
            repo_root,
            backend.as_mut(),
            stop_rx,
            refresh,
        )
    }))
    .unwrap_or_else(|panic| {
        Err(WatcherFailure::new(
            WatcherFailurePhase::Worker,
            WatcherFailureKind::Panic,
            panic_message(panic),
        ))
    });
    if let Err(failure) = &primary {
        replace_state(&state, RepoWatcherWorkerState::Failed(failure.clone()));
    }
    let cleanup = stop_backend(backend.as_mut());
    let result = match (primary, cleanup) {
        (Err(failure), Err(cleanup)) => Err(failure.with_cleanup(cleanup.to_string())),
        (Err(failure), Ok(())) => Err(failure),
        (Ok(()), Err(cleanup)) => Err(cleanup),
        (Ok(()), Ok(())) => Ok(()),
    };
    if let Err(failure) = &result {
        replace_state(&state, RepoWatcherWorkerState::Failed(failure.clone()));
    }
    result
}

fn consume_loop(
    sync: Arc<SyncManager>,
    repo_name: &str,
    repo_id: RepoId,
    repo_root: PathBuf,
    backend: &mut dyn FsWatcherBackend,
    stop_rx: mpsc::Receiver<()>,
    refresh: Option<WatcherRefreshCallback>,
) -> Result<(), WatcherFailure> {
    loop {
        if stop_rx.try_recv().is_ok() {
            return Ok(());
        }
        let signal = catch_unwind(AssertUnwindSafe(|| {
            backend.recv(Duration::from_millis(100))
        }))
        .map_err(|panic| panic_failure(WatcherFailurePhase::Receive, panic))?
        .map_err(|error| {
            WatcherFailure::new(
                WatcherFailurePhase::Receive,
                WatcherFailureKind::Backend,
                error.to_string(),
            )
        })?;
        match signal {
            Some(BackendSignal::Hints(batch)) => catch_unwind(AssertUnwindSafe(|| {
                dispatch::dispatch_batch(
                    &sync,
                    repo_name,
                    repo_id,
                    &repo_root,
                    batch.hints(),
                    refresh.as_ref(),
                )
            }))
            .map_err(|panic| panic_failure(WatcherFailurePhase::Dispatch, panic))?
            .map_err(|error| {
                WatcherFailure::new(
                    WatcherFailurePhase::Dispatch,
                    WatcherFailureKind::Dispatch,
                    error.to_string(),
                )
            })?,
            Some(BackendSignal::Reconcile(token)) => {
                catch_unwind(AssertUnwindSafe(|| {
                    rescan_and_notify(&sync, repo_name, repo_id, refresh.as_ref())
                }))
                .map_err(|panic| panic_failure(WatcherFailurePhase::Reconcile, panic))?
                .map_err(|error| {
                    WatcherFailure::new(
                        WatcherFailurePhase::Reconcile,
                        WatcherFailureKind::Scan,
                        error.to_string(),
                    )
                })?;
                let _clean =
                    catch_unwind(AssertUnwindSafe(|| backend.complete_reconcile(token)))
                        .map_err(|panic| panic_failure(WatcherFailurePhase::Reconcile, panic))?;
            }
            Some(BackendSignal::Terminal) => {
                return Err(WatcherFailure::new(
                    WatcherFailurePhase::Receive,
                    WatcherFailureKind::Backend,
                    "watcher backend producer stopped",
                ));
            }
            None => {}
        }
    }
}

pub(crate) fn stop_backend(backend: &mut dyn FsWatcherBackend) -> Result<(), WatcherFailure> {
    catch_unwind(AssertUnwindSafe(|| backend.stop()))
        .map_err(|panic| panic_failure(WatcherFailurePhase::Shutdown, panic))?
        .map_err(|error| {
            WatcherFailure::new(
                WatcherFailurePhase::Shutdown,
                WatcherFailureKind::Backend,
                error.to_string(),
            )
        })
}

fn rescan_and_notify(
    sync: &SyncManager,
    repo_name: &str,
    repo_id: RepoId,
    callback: Option<&WatcherRefreshCallback>,
) -> anyhow::Result<()> {
    let (refreshed_repo_id, refreshed_path) = sync.force_dir_refresh(repo_name, repo_id, "")?;
    if let Some(callback) = callback {
        callback(WatcherRefresh::new(
            refreshed_repo_id,
            refreshed_path,
            super::WatcherRefreshKind::DirectoryChanged,
            false,
        ));
    }
    Ok(())
}

fn replace_state(state: &RwLock<RepoWatcherWorkerState>, next: RepoWatcherWorkerState) {
    match state.write() {
        Ok(mut state) => *state = next,
        Err(poisoned) => *poisoned.into_inner() = next,
    }
}

pub(crate) fn panic_message(panic: Box<dyn Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        "watcher worker panicked".to_owned()
    }
}

fn panic_failure(phase: WatcherFailurePhase, panic: Box<dyn Any + Send>) -> WatcherFailure {
    WatcherFailure::new(phase, WatcherFailureKind::Panic, panic_message(panic))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::watcher::backend::ReconcileToken;
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
        fn recv(
            &self,
            _timeout: Duration,
        ) -> Result<Option<BackendSignal>, super::super::WatcherError> {
            Err(super::super::WatcherError::WatcherInitFailed(
                "injected receive failure".to_owned(),
            ))
        }

        fn complete_reconcile(&self, _token: ReconcileToken) -> bool {
            false
        }

        fn stop(&mut self) -> Result<(), super::super::WatcherError> {
            Err(super::super::WatcherError::WatcherInitFailed(
                "injected cleanup failure".to_owned(),
            ))
        }
    }

    impl FsWatcherBackend for StopPanickingBackend {
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
    }

    impl FsWatcherBackend for BlockingCleanupBackend {
        fn recv(
            &self,
            _timeout: Duration,
        ) -> Result<Option<BackendSignal>, super::super::WatcherError> {
            Err(super::super::WatcherError::WatcherInitFailed(
                "terminal receive failure".to_owned(),
            ))
        }

        fn complete_reconcile(&self, _token: ReconcileToken) -> bool {
            false
        }

        fn stop(&mut self) -> Result<(), super::super::WatcherError> {
            self.entered.send(()).expect("announce cleanup");
            self.release.recv().expect("release cleanup");
            Ok(())
        }
    }

    impl FsWatcherBackend for PanickingBackend {
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
    }

    #[test]
    fn worker_panic_becomes_typed_failure_and_stops_backend() {
        let (_dir, _repo, sync, repo_name, repo_id, repo_root) =
            super::super::dispatch_test_support::new_sync().expect("watcher fixture");
        let stopped = Arc::new(AtomicBool::new(false));
        let state = Arc::new(RwLock::new(RepoWatcherWorkerState::Running));
        let (_stop_tx, stop_rx) = mpsc::channel();

        let failure = run(WorkerInput {
            sync,
            repo_name,
            repo_id,
            repo_root,
            backend: Box::new(PanickingBackend {
                stopped: stopped.clone(),
            }),
            stop_rx,
            refresh: None,
            state: state.clone(),
        })
        .expect_err("panic must become terminal failure");

        assert_eq!(failure.kind, WatcherFailureKind::Panic);
        assert!(failure.primary.contains("injected watcher panic"));
        assert!(stopped.load(Ordering::SeqCst));
        assert!(matches!(
            state.read().expect("state").clone(),
            RepoWatcherWorkerState::Failed(observed) if observed == failure
        ));
    }

    #[test]
    fn consumer_failure_preserves_primary_and_cleanup() {
        let (_dir, _repo, sync, repo_name, repo_id, repo_root) =
            super::super::dispatch_test_support::new_sync().expect("watcher fixture");
        let state = Arc::new(RwLock::new(RepoWatcherWorkerState::Running));
        let (_stop_tx, stop_rx) = mpsc::channel();

        let failure = run(WorkerInput {
            sync,
            repo_name,
            repo_id,
            repo_root,
            backend: Box::new(FailingBackend),
            stop_rx,
            refresh: None,
            state,
        })
        .expect_err("receive and cleanup failure must escape");

        assert_eq!(failure.phase, WatcherFailurePhase::Receive);
        assert_eq!(failure.kind, WatcherFailureKind::Backend);
        assert!(failure.primary.contains("injected receive failure"));
        assert_eq!(failure.cleanup.len(), 1);
        assert!(failure.cleanup[0].contains("injected cleanup failure"));
    }

    #[test]
    fn full_rescan_emits_repo_scoped_refresh() {
        let (_dir, _repo, sync, repo_name, repo_id, _repo_root) =
            super::super::dispatch_test_support::new_sync().expect("watcher fixture");
        let refreshes = Arc::new(std::sync::Mutex::new(Vec::new()));
        let observed = refreshes.clone();
        let callback: WatcherRefreshCallback = Arc::new(move |refresh| {
            observed.lock().expect("refresh lock").push(refresh);
        });

        rescan_and_notify(&sync, &repo_name, repo_id, Some(&callback))
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
        let state = Arc::new(RwLock::new(RepoWatcherWorkerState::Running));
        let (stop_tx, stop_rx) = mpsc::channel();
        stop_tx.send(()).expect("request stop");

        let failure = run(WorkerInput {
            sync,
            repo_name,
            repo_id,
            repo_root,
            backend: Box::new(StopPanickingBackend),
            stop_rx,
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
        let state = Arc::new(RwLock::new(RepoWatcherWorkerState::Running));
        let observed = state.clone();
        let (_stop_tx, stop_rx) = mpsc::channel();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();

        let join = std::thread::spawn(move || {
            run(WorkerInput {
                sync,
                repo_name,
                repo_id,
                repo_root,
                backend: Box::new(BlockingCleanupBackend {
                    entered: entered_tx,
                    release: release_rx,
                }),
                stop_rx,
                refresh: None,
                state,
            })
        });
        entered_rx.recv().expect("cleanup entered");

        assert!(matches!(
            observed.read().expect("state").clone(),
            RepoWatcherWorkerState::Failed(failure)
                if failure.phase == WatcherFailurePhase::Receive
        ));

        release_tx.send(()).expect("release cleanup");
        join.join()
            .expect("worker join")
            .expect_err("receive failure remains terminal");
    }
}

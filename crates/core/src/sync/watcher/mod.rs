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
mod registry;

use crate::models::RepoId;
use crate::protocol::ServerMessage;
use crate::sync::SyncManager;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::warn;

pub type WatcherCallback = Arc<dyn Fn(ServerMessage) + Send + Sync>;
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("WatcherInitFailed: {0}")]
    WatcherInitFailed(String),
    #[error("Watcher already running for repo {0}")]
    AlreadyRunning(RepoId),
    #[error("Watcher registry lock poisoned")]
    RegistryPoisoned,
    #[error("Watcher debounce window must be non-zero")]
    ZeroDebounce,
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

pub fn start_repo_watcher(
    sync: Arc<SyncManager>,
    repo_name: &str,
    debounce: Option<Duration>,
    callback: Option<WatcherCallback>,
) -> Result<RepoId, WatcherError> {
    let debounce = debounce.unwrap_or(DEFAULT_DEBOUNCE);
    if debounce.is_zero() {
        return Err(WatcherError::ZeroDebounce);
    }
    let info = sync
        .repo
        .get_repo_info_for(None, Some(repo_name))?
        .ok_or_else(|| {
            WatcherError::WatcherInitFailed(format!("Repository missing: {repo_name}"))
        })?;
    if registry::is_running(info.uuid)? {
        return Err(WatcherError::AlreadyRunning(info.uuid));
    }
    crate::sync::scan::scan_local_repo(&sync.repo, &sync.vfs, repo_name)?;
    let repo_root = sync.repo.local_repo_workspace_root(repo_name)?;
    let repo_root = std::fs::canonicalize(&repo_root).map_err(|err| {
        WatcherError::WatcherInitFailed(format!(
            "Failed to canonicalize watcher root {:?}: {err}",
            repo_root
        ))
    })?;
    let mut backend = backend::desktop_backend(&repo_root, debounce)
        .map_err(|err| WatcherError::WatcherInitFailed(err.to_string()))?;
    let (stop_tx, stop_rx) = std::sync::mpsc::channel();
    let sync_clone = sync.clone();
    let name = repo_name.to_string();
    let cb = callback.clone();
    let join = std::thread::spawn(move || {
        run_loop(
            sync_clone,
            &name,
            info.uuid,
            repo_root,
            &mut *backend,
            stop_rx,
            cb,
        )
    });
    let handle = registry::WatcherHandle { stop_tx, join };
    register_handle_or_stop_rejected(info.uuid, handle)?;
    Ok(info.uuid)
}

pub fn stop_repo_watcher(repo_id: RepoId) -> Result<(), WatcherError> {
    let Some(handle) = registry::begin_stop(repo_id)? else {
        return Ok(());
    };
    let stop_result = stop_handle(handle);
    let finish_result = registry::finish_stop(repo_id);
    match (stop_result, finish_result) {
        (Err(err), _) => Err(err),
        (Ok(()), Err(err)) => Err(err),
        (Ok(()), Ok(())) => Ok(()),
    }
}

pub(crate) fn is_repo_watcher_running(repo_id: RepoId) -> Result<bool, WatcherError> {
    registry::is_running(repo_id)
}

fn stop_handle(handle: registry::WatcherHandle) -> Result<(), WatcherError> {
    let _ = handle.stop_tx.send(());
    handle
        .join
        .join()
        .map_err(|_| WatcherError::WatcherInitFailed("watcher thread panicked".into()))?
}

fn register_handle_or_stop_rejected(
    repo_id: RepoId,
    handle: registry::WatcherHandle,
) -> Result<(), WatcherError> {
    if let Err((err, rejected)) = registry::insert_or_reject(repo_id, handle) {
        stop_handle(rejected)?;
        return Err(err);
    }
    Ok(())
}

fn run_loop(
    sync: Arc<SyncManager>,
    repo_name: &str,
    repo_id: RepoId,
    repo_root: std::path::PathBuf,
    backend: &mut dyn backend::FsWatcherBackend,
    stop_rx: std::sync::mpsc::Receiver<()>,
    callback: Option<WatcherCallback>,
) -> Result<(), WatcherError> {
    let consume_result = consume_loop(
        sync, repo_name, repo_id, repo_root, backend, stop_rx, callback,
    );
    let stop_result = backend.stop();
    match (consume_result, stop_result) {
        (Err(primary), Err(cleanup)) => {
            warn!(
                primary_error = %primary,
                cleanup_error = %cleanup,
                "watcher backend cleanup failed after consumer failure"
            );
            Err(primary)
        }
        (Err(primary), Ok(())) => Err(primary),
        (Ok(()), Err(cleanup)) => Err(cleanup),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn consume_loop(
    sync: Arc<SyncManager>,
    repo_name: &str,
    repo_id: RepoId,
    repo_root: std::path::PathBuf,
    backend: &mut dyn backend::FsWatcherBackend,
    stop_rx: std::sync::mpsc::Receiver<()>,
    callback: Option<WatcherCallback>,
) -> Result<(), WatcherError> {
    loop {
        if stop_rx.try_recv().is_ok() {
            return Ok(());
        }
        match backend.recv(Duration::from_millis(100))? {
            Some(BackendSignal::Hints(batch)) => {
                dispatch::dispatch_batch(
                    &sync,
                    repo_name,
                    repo_id,
                    &repo_root,
                    batch.hints(),
                    callback.as_ref(),
                )?;
            }
            Some(BackendSignal::Reconcile(token)) => {
                rescan_and_notify(&sync, repo_name, repo_id, callback.as_ref())?;
                let _clean = backend.complete_reconcile(token);
            }
            Some(BackendSignal::Terminal) => {
                return Err(WatcherError::WatcherInitFailed(
                    "watcher backend producer stopped".into(),
                ));
            }
            None => {}
        }
    }
}

fn rescan_and_notify(
    sync: &SyncManager,
    repo_name: &str,
    repo_id: RepoId,
    callback: Option<&WatcherCallback>,
) -> Result<(), WatcherError> {
    let (refreshed_repo_id, refreshed_path) = sync.force_dir_refresh(repo_name, repo_id, "")?;
    if let Some(cb) = callback {
        cb(crate::sync::pending::dir_changed_message(
            refreshed_repo_id,
            &refreshed_path,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, mpsc};

    struct FailingBackend {
        stopped: Arc<AtomicBool>,
    }

    impl backend::FsWatcherBackend for FailingBackend {
        fn recv(&self, _timeout: Duration) -> Result<Option<BackendSignal>, WatcherError> {
            Err(WatcherError::WatcherInitFailed(
                "injected backend failure".into(),
            ))
        }

        fn complete_reconcile(&self, _token: ReconcileToken) -> bool {
            false
        }

        fn stop(&mut self) -> Result<(), WatcherError> {
            self.stopped.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    fn stop_observed_handle() -> (
        registry::WatcherHandle,
        mpsc::Receiver<()>,
        mpsc::Receiver<()>,
    ) {
        let (stop_tx, stop_rx) = mpsc::channel();
        let (stopped_tx, stopped_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let join = std::thread::spawn(move || {
            stop_rx
                .recv()
                .map_err(|err| WatcherError::WatcherInitFailed(err.to_string()))?;
            stopped_tx
                .send(())
                .map_err(|err| WatcherError::WatcherInitFailed(err.to_string()))?;
            let _ = done_tx.send(());
            Ok(())
        });
        (
            registry::WatcherHandle { stop_tx, join },
            stopped_rx,
            done_rx,
        )
    }

    #[test]
    fn rejected_registration_stops_and_joins_rejected_handle() {
        let repo_id = uuid::Uuid::from_u128(29);
        let _ = registry::begin_stop(repo_id);
        let _ = registry::finish_stop(repo_id);

        let (first, first_stopped, _) = stop_observed_handle();
        register_handle_or_stop_rejected(repo_id, first).expect("register first watcher");

        let (second, second_stopped, second_done) = stop_observed_handle();
        let err = register_handle_or_stop_rejected(repo_id, second)
            .expect_err("duplicate watcher must be rejected");
        assert!(matches!(err, WatcherError::AlreadyRunning(id) if id == repo_id));
        second_stopped
            .recv()
            .expect("rejected watcher received stop");
        second_done.recv().expect("rejected watcher completed");

        let original = registry::begin_stop(repo_id)
            .expect("begin stop first watcher")
            .expect("first watcher registered");
        stop_handle(original).expect("stop first watcher");
        first_stopped.recv().expect("first watcher received stop");
        registry::finish_stop(repo_id).expect("finish stop first watcher");
    }

    #[test]
    fn consumer_failure_still_stops_backend() {
        let (_dir, _repo, sync, repo_name, repo_id, repo_root) =
            dispatch_test_support::new_sync().expect("watcher fixture");
        let stopped = Arc::new(AtomicBool::new(false));
        let mut backend = FailingBackend {
            stopped: stopped.clone(),
        };
        let (_stop_tx, stop_rx) = mpsc::channel();

        let error = run_loop(
            sync,
            &repo_name,
            repo_id,
            repo_root,
            &mut backend,
            stop_rx,
            None,
        )
        .expect_err("injected receive failure must escape");

        assert!(error.to_string().contains("injected backend failure"));
        assert!(stopped.load(Ordering::SeqCst));
    }

    #[test]
    fn full_rescan_emits_repo_scoped_dir_changed_message() {
        let (_dir, _repo, sync, repo_name, repo_id, _repo_root) =
            dispatch_test_support::new_sync().expect("watcher fixture");
        let messages = Arc::new(Mutex::new(Vec::new()));
        let callback_messages = messages.clone();
        let callback: WatcherCallback = Arc::new(move |message| {
            callback_messages
                .lock()
                .expect("messages lock")
                .push(message);
        });

        rescan_and_notify(&sync, &repo_name, repo_id, Some(&callback))
            .expect("full watcher rescan");

        let messages = messages.lock().expect("messages lock");
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            ServerMessage::FsChangeDetected {
                repo_id: message_repo_id,
                path,
                change_type,
                ..
            } => {
                assert_eq!(*message_repo_id, Some(repo_id));
                assert_eq!(path, "");
                assert_eq!(change_type, "dir_changed");
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }
}

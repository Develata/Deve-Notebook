//! plan_ref:
//!   - 04_storage#watcher-contract

pub(crate) mod backend;
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
    loop {
        if stop_rx.try_recv().is_ok() {
            return backend.stop();
        }
        match backend.recv(Duration::from_millis(100))? {
            Some(backend::BackendBatch::Events(events)) => {
                dispatch::dispatch_batch(
                    &sync,
                    repo_name,
                    repo_id,
                    &repo_root,
                    events,
                    callback.as_ref(),
                )?;
            }
            Some(backend::BackendBatch::Rescan) => {
                crate::sync::scan::scan_local_repo(&sync.repo, &sync.vfs, repo_name)?
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use std::sync::mpsc;

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
}

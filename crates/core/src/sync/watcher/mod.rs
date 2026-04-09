//! plan_ref:
//!   - 04_storage#watcher-contract

pub(crate) mod backend;
mod dispatch;
mod filter;
mod registry;
pub(crate) mod suppressor;

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
    #[error("Watcher path escaped repo root: {0:?}")]
    PathEscaped(std::path::PathBuf),
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
    registry::insert(info.uuid, registry::WatcherHandle { stop_tx, join })?;
    Ok(info.uuid)
}

pub fn stop_repo_watcher(repo_id: RepoId) -> Result<(), WatcherError> {
    let Some(handle) = registry::remove(repo_id)? else {
        return Ok(());
    };
    let _ = handle.stop_tx.send(());
    handle
        .join
        .join()
        .map_err(|_| WatcherError::WatcherInitFailed("watcher thread panicked".into()))?
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

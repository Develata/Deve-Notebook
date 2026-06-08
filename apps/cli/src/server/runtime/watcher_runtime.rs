//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 07_network#server-ws-runtime
//!
//! File watcher runtime assembly.

use crate::server::setup;
use anyhow::Result;
use deve_core::models::RepoId;
use deve_core::protocol::ServerMessage;
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tokio::sync::broadcast;

pub(crate) struct FileWatcherRuntimeGuard {
    repo_ids: Vec<RepoId>,
}

impl Drop for FileWatcherRuntimeGuard {
    fn drop(&mut self) {
        for repo_id in self.repo_ids.drain(..) {
            let _ = deve_core::sync::watcher::stop_repo_watcher(repo_id);
        }
    }
}

pub(crate) fn start_file_watchers(
    sync_manager: Arc<SyncManager>,
    tx: broadcast::Sender<ServerMessage>,
) -> Result<FileWatcherRuntimeGuard> {
    Ok(FileWatcherRuntimeGuard {
        repo_ids: setup::start_file_watchers(sync_manager, tx)?,
    })
}

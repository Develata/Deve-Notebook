//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 07_network#server-ws-runtime
//!
//! File watcher runtime assembly.

use crate::server::setup;
use crate::watcher_runtime::OwnedWatcherHandles;
use anyhow::Result;
use deve_core::protocol::ServerMessage;
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tokio::sync::broadcast;

pub(crate) struct FileWatcherRuntimeGuard {
    handles: OwnedWatcherHandles,
}

impl FileWatcherRuntimeGuard {
    pub(crate) fn shutdown(self) -> Result<()> {
        Ok(self.handles.shutdown()?)
    }
}

pub(crate) fn start_file_watchers(
    sync_manager: Arc<SyncManager>,
    tx: broadcast::Sender<ServerMessage>,
) -> Result<FileWatcherRuntimeGuard> {
    Ok(FileWatcherRuntimeGuard {
        handles: setup::start_file_watchers(sync_manager, tx)?,
    })
}

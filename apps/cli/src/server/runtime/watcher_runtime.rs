//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 07_network#server-ws-runtime
//!
//! Server watcher composition adapter. Lifecycle ownership stays in the
//! supervisor; AppState receives only its read-only runtime view.

mod slot;
mod supervisor;

use crate::server::setup;
use anyhow::Result;
use deve_core::protocol::ServerMessage;
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tokio::sync::broadcast;

pub(crate) use slot::{
    MountAdmissionError, MountAdmissionToken, WatcherRuntimeAggregate, WatcherRuntimeView,
};
#[cfg(test)]
pub(crate) use slot::{RepoMountState, WatcherRuntimeAggregateStatus};
pub(crate) use supervisor::WatcherSupervisor;

pub(crate) fn start_file_watchers(
    sync_manager: Arc<SyncManager>,
    tx: broadcast::Sender<ServerMessage>,
) -> Result<WatcherSupervisor> {
    let starts = setup::file_watcher_starts(sync_manager, tx)?;
    Ok(WatcherSupervisor::start_all(starts)?)
}

#[cfg(test)]
mod tests;

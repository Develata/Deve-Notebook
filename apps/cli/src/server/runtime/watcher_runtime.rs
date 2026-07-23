//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 07_network#server-ws-runtime
//!
//! Server watcher composition adapter. Lifecycle ownership stays in the
//! supervisor; AppState receives only its read-only runtime view.

mod bootstrap;
mod error;
mod lifecycle;
mod mount;
mod refresh_route;
mod slot;
mod supervisor;
mod view;

use crate::server::setup;
use anyhow::Result;
use deve_core::protocol::ServerMessage;
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tokio::sync::broadcast;

pub(crate) use error::WatcherLifecycleError;
pub(crate) use lifecycle::WatcherMountReservation;
#[cfg(test)]
pub(crate) use slot::RepoMountState;
pub(crate) use supervisor::WatcherSupervisor;
#[cfg(test)]
pub(crate) use view::WatcherRuntimeAggregateStatus;
pub(crate) use view::{
    MountAdmissionError, MountAdmissionToken, WatcherRuntimeAggregate, WatcherRuntimeView,
};

pub(crate) fn start_file_watchers(
    sync_manager: Arc<SyncManager>,
    tx: broadcast::Sender<ServerMessage>,
) -> Result<WatcherSupervisor> {
    let starts = setup::file_watcher_starts(sync_manager).map_err(|error| {
        error::WatcherSupervisorStartError::new(
            error::WatcherHostFatalKind::RuntimeCoordinationFailure,
            None,
            format!("watcher bootstrap inventory failed: {error}"),
        )
    })?;
    let publisher = Arc::new(move |refresh| {
        let _ = tx.send(setup::watcher_refresh_message(refresh));
    });
    Ok(WatcherSupervisor::start_all(starts, publisher)?)
}

#[cfg(test)]
mod tests;

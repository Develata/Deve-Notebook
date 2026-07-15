//! plan_ref:
//!   - 07_network#projection-recovery-contract
//!   - 09_web_thin_client_ledger#projection-recovery-coordinator
//!
//! Loss-triggered transport recovery for the bounded Web incoming ring.

use super::WsService;
use crate::api::connection::ConnectionControl;
use leptos::prelude::{GetUntracked, Set};

impl WsService {
    /// Retire the current session after a local incoming-ring gap.
    ///
    /// The epoch guard coalesces the editor and core consumers into one
    /// reconnect request. Writer readiness is revoked synchronously before the
    /// connection manager observes the command.
    pub(crate) fn request_reconnect_for_resync(&self, connection_epoch: u64) -> bool {
        self.clear_writer_ready();
        if self.reconnect_requested_epoch.get_untracked() == Some(connection_epoch) {
            return false;
        }
        self.set_reconnect_requested_epoch
            .set(Some(connection_epoch));
        if let Err(error) = self
            .connection_control_tx
            .unbounded_send(ConnectionControl::ReconnectForResync { connection_epoch })
        {
            leptos::logging::error!("projection resync reconnect request failed: {error:?}");
        }
        true
    }

    pub(crate) fn reconnect_for_resync_pending(&self, connection_epoch: u64) -> bool {
        self.reconnect_requested_epoch.get_untracked() == Some(connection_epoch)
    }
}

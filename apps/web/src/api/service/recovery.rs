//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 07_network#projection-recovery-contract
//!   - 09_web_thin_client_ledger#projection-recovery-coordinator
//!
//! Loss-triggered transport recovery for the bounded Web incoming ring.

use super::WsService;
use crate::api::ConnectionStatus;
use crate::api::connection::ConnectionControl;
use crate::api::outbound_admission::OutboundAdmissionFailure;
use leptos::prelude::{GetUntracked, Set};

impl WsService {
    pub(super) fn handle_outbound_admission_failure(&self, failure: OutboundAdmissionFailure) {
        if !self.lifecycle.is_active() {
            return;
        }
        let connection_epoch = self.connection_epoch.get_untracked();
        let already_requested =
            self.outbound_retirement_requested_epoch.get_untracked() == Some(connection_epoch);

        self.clear_writer_ready();
        if matches!(
            self.status.get_untracked(),
            ConnectionStatus::Connected
                | ConnectionStatus::Connecting
                | ConnectionStatus::Disconnected
        ) {
            self.reset_node_role_state(false);
            self.set_status.set(ConnectionStatus::Disconnected);
        }

        if already_requested {
            return;
        }
        leptos::logging::error!(
            "web_outbound_admission_rejected kind={} class={} connection_epoch={}",
            failure.kind.label(),
            failure.message_class.label(),
            connection_epoch
        );
        self.set_outbound_retirement_requested_epoch
            .set(Some(connection_epoch));
        if self
            .connection_control_tx
            .unbounded_send(ConnectionControl::RetireOutboundAdmission {
                observed_connection_epoch: connection_epoch,
            })
            .is_err()
        {
            leptos::logging::error!(
                "web_outbound_admission_retire_control_closed connection_epoch={}",
                connection_epoch
            );
        }
    }

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

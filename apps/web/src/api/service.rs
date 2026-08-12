//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 08_auth#unauthorized-handling
//!   - 18_release#runtime-observability
//!   - 08_auth#unauthorized-disconnected-ui
//!

use deve_core::protocol::{ClientMessage, ServerMessage};
#[cfg(test)]
use futures::channel::mpsc::UnboundedReceiver;
use futures::channel::mpsc::{UnboundedSender, unbounded};
use leptos::prelude::*;
use std::collections::VecDeque;
#[cfg(test)]
use std::sync::{Arc, Mutex};

use self::service_ping::spawn_ping_loop;
use super::connection::{
    ConnectionControl, ConnectionLifecycle, ConnectionManagerSignals, spawn_connection_manager,
};
use super::connection_role::WatcherHealthSnapshot;
use super::incoming::{IncomingBatch, messages_since};
use super::status::ConnectionStatus;
use super::write_gate::WriterReadyResetSignals;
use super::writer_id::new_writer_session_nonce;

mod readiness;
mod recovery;
mod service_ping;
mod source_control;
#[cfg(test)]
mod test_support;

#[derive(Clone, Debug, PartialEq, Eq)]
struct WorkspaceIngestionBlocker {
    connection_epoch: u64,
    repo_id: String,
    scope_nonce: u64,
}

#[derive(Clone)]
pub struct WsService {
    lifecycle: ConnectionLifecycle,
    pub status: ReadSignal<ConnectionStatus>,
    set_status: WriteSignal<ConnectionStatus>,
    pub writer_ready_repo_id: ReadSignal<Option<String>>,
    set_writer_ready_repo_id: WriteSignal<Option<String>>,
    writer_ready_scope_nonce: ReadSignal<Option<u64>>,
    set_writer_ready_scope_nonce: WriteSignal<Option<u64>>,
    pub writer_client_id: ReadSignal<Option<u64>>,
    set_writer_client_id: WriteSignal<Option<u64>>,
    pub endpoint: ReadSignal<String>,
    pub node_role: ReadSignal<String>,
    set_node_role: WriteSignal<String>,
    pub source_control_authority: ReadSignal<String>,
    set_source_control_authority: WriteSignal<String>,
    pub host_file_copy_absolute_path: ReadSignal<bool>,
    set_host_file_copy_absolute_path: WriteSignal<bool>,
    pub host_file_reveal_in_system_explorer: ReadSignal<bool>,
    set_host_file_reveal_in_system_explorer: WriteSignal<bool>,
    pub(crate) watcher_health: ReadSignal<WatcherHealthSnapshot>,
    set_watcher_health: WriteSignal<WatcherHealthSnapshot>,
    pub node_role_probe_failed: ReadSignal<bool>,
    set_node_role_probe_failed: WriteSignal<bool>,
    workspace_ingestion_blocker: ReadSignal<Option<WorkspaceIngestionBlocker>>,
    set_workspace_ingestion_blocker: WriteSignal<Option<WorkspaceIngestionBlocker>>,
    pub msg_seq: ReadSignal<u64>,
    pub connection_epoch: ReadSignal<u64>,
    reconnect_requested_epoch: ReadSignal<Option<u64>>,
    set_reconnect_requested_epoch: WriteSignal<Option<u64>>,
    external_apply_request_id: ReadSignal<Option<String>>,
    set_external_apply_request_id: WriteSignal<Option<String>>,
    writer_session_nonce: u64,
    msg_queue: ReadSignal<VecDeque<(u64, u64, ServerMessage)>>,
    tx: UnboundedSender<ClientMessage>,
    connection_control_tx: UnboundedSender<ConnectionControl>,
    #[cfg(test)]
    test_rx: Option<Arc<Mutex<UnboundedReceiver<ClientMessage>>>>,
    #[cfg(test)]
    test_connection_control_rx: Option<Arc<Mutex<UnboundedReceiver<ConnectionControl>>>>,
}

impl WsService {
    pub fn new() -> Self {
        let (status, set_status) = signal(ConnectionStatus::Disconnected);
        let (writer_ready_repo_id, set_writer_ready_repo_id) = signal(None::<String>);
        let (writer_ready_scope_nonce, set_writer_ready_scope_nonce) = signal(None::<u64>);
        let (writer_client_id, set_writer_client_id) = signal(None::<u64>);
        let (msg_seq, set_msg_seq) = signal(0u64);
        let (connection_epoch, set_connection_epoch) = signal(0u64);
        let (reconnect_requested_epoch, set_reconnect_requested_epoch) = signal(None::<u64>);
        let (external_apply_request_id, set_external_apply_request_id) = signal(None::<String>);
        let (msg_queue, set_msg_queue) = signal(VecDeque::<(u64, u64, ServerMessage)>::new());
        let (endpoint, set_endpoint) = signal(String::new());
        let (node_role, set_node_role) = signal(String::new());
        let (source_control_authority, set_source_control_authority) =
            signal("unknown".to_string());
        let (host_file_copy_absolute_path, set_host_file_copy_absolute_path) = signal(false);
        let (host_file_reveal_in_system_explorer, set_host_file_reveal_in_system_explorer) =
            signal(false);
        let (watcher_health, set_watcher_health) = signal(WatcherHealthSnapshot::default());
        let (node_role_probe_failed, set_node_role_probe_failed) = signal(false);
        let (workspace_ingestion_blocker, set_workspace_ingestion_blocker) =
            signal(None::<WorkspaceIngestionBlocker>);
        let (tx, rx) = unbounded::<ClientMessage>();
        let (connection_control_tx, connection_control_rx) = unbounded::<ConnectionControl>();
        let lifecycle = ConnectionLifecycle::new();
        let cleanup_lifecycle = lifecycle.clone();
        on_cleanup(move || cleanup_lifecycle.shutdown());
        let writer_ready_reset = WriterReadyResetSignals::new(
            set_writer_ready_repo_id,
            set_writer_ready_scope_nonce,
            set_writer_client_id,
        );

        spawn_connection_manager(
            rx,
            connection_control_rx,
            ConnectionManagerSignals {
                lifecycle: lifecycle.clone(),
                current_status: status,
                set_status,
                set_msg_seq,
                set_msg_queue,
                current_connection_epoch: connection_epoch,
                set_connection_epoch,
                set_endpoint,
                set_node_role,
                set_source_control_authority,
                set_host_file_copy_absolute_path,
                set_host_file_reveal_in_system_explorer,
                set_watcher_health,
                set_node_role_probe_failed,
                writer_ready_reset,
            },
        );

        spawn_ping_loop(status, tx.clone());

        Self {
            lifecycle,
            status,
            set_status,
            writer_ready_repo_id,
            set_writer_ready_repo_id,
            writer_ready_scope_nonce,
            set_writer_ready_scope_nonce,
            writer_client_id,
            set_writer_client_id,
            endpoint,
            node_role,
            set_node_role,
            source_control_authority,
            set_source_control_authority,
            host_file_copy_absolute_path,
            set_host_file_copy_absolute_path,
            host_file_reveal_in_system_explorer,
            set_host_file_reveal_in_system_explorer,
            watcher_health,
            set_watcher_health,
            node_role_probe_failed,
            set_node_role_probe_failed,
            workspace_ingestion_blocker,
            set_workspace_ingestion_blocker,
            msg_seq,
            connection_epoch,
            reconnect_requested_epoch,
            set_reconnect_requested_epoch,
            external_apply_request_id,
            set_external_apply_request_id,
            writer_session_nonce: new_writer_session_nonce(),
            msg_queue,
            tx,
            connection_control_tx,
            #[cfg(test)]
            test_rx: None,
            #[cfg(test)]
            test_connection_control_rx: None,
        }
    }

    pub fn send(&self, msg: ClientMessage) {
        if let Err(e) = self.tx.unbounded_send(msg) {
            leptos::logging::error!("消息入队失败: {:?}", e);
        }
    }

    pub(crate) fn request_native_endpoint_rebind(&self) {
        if let Err(error) = self
            .connection_control_tx
            .unbounded_send(ConnectionControl::RebindNativeEndpoint)
        {
            leptos::logging::error!("Native endpoint rebind request failed: {error:?}");
            self.mark_native_service_offline();
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        self.lifecycle.is_active()
    }

    pub(crate) fn active_endpoint_epoch(&self) -> Option<(String, u64)> {
        Some((
            self.lifecycle.try_get(self.endpoint)?,
            self.lifecycle.try_get(self.connection_epoch)?,
        ))
    }

    pub(crate) fn messages_since(&self, after_seq: u64) -> IncomingBatch {
        self.msg_queue
            .with_untracked(|queue| messages_since(queue, after_seq))
    }
}

pub(crate) use self::readiness::is_current_connection_message;

#[cfg(test)]
mod tests;

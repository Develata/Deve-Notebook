//! plan_ref:
//!   - 08_auth#unauthorized-handling
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
use super::connection::{ConnectionLifecycle, ConnectionManagerSignals, spawn_connection_manager};
use super::status::ConnectionStatus;
use super::write_gate::WriterReadyResetSignals;
use super::writer_id::new_writer_session_nonce;

mod readiness;
mod service_ping;
#[cfg(test)]
mod test_support;

#[derive(Clone)]
pub struct WsService {
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
    pub node_role_probe_failed: ReadSignal<bool>,
    set_node_role_probe_failed: WriteSignal<bool>,
    pub msg_seq: ReadSignal<u64>,
    pub connection_epoch: ReadSignal<u64>,
    writer_session_nonce: u64,
    msg_queue: ReadSignal<VecDeque<(u64, u64, ServerMessage)>>,
    tx: UnboundedSender<ClientMessage>,
    #[cfg(test)]
    test_rx: Option<Arc<Mutex<UnboundedReceiver<ClientMessage>>>>,
}

impl WsService {
    pub fn new() -> Self {
        let (status, set_status) = signal(ConnectionStatus::Disconnected);
        let (writer_ready_repo_id, set_writer_ready_repo_id) = signal(None::<String>);
        let (writer_ready_scope_nonce, set_writer_ready_scope_nonce) = signal(None::<u64>);
        let (writer_client_id, set_writer_client_id) = signal(None::<u64>);
        let (msg_seq, set_msg_seq) = signal(0u64);
        let (connection_epoch, set_connection_epoch) = signal(0u64);
        let (msg_queue, set_msg_queue) = signal(VecDeque::<(u64, u64, ServerMessage)>::new());
        let (endpoint, set_endpoint) = signal(String::new());
        let (node_role, set_node_role) = signal(String::new());
        let (source_control_authority, set_source_control_authority) =
            signal("unknown".to_string());
        let (host_file_copy_absolute_path, set_host_file_copy_absolute_path) = signal(false);
        let (host_file_reveal_in_system_explorer, set_host_file_reveal_in_system_explorer) =
            signal(false);
        let (node_role_probe_failed, set_node_role_probe_failed) = signal(false);
        let (tx, rx) = unbounded::<ClientMessage>();
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
            ConnectionManagerSignals {
                lifecycle,
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
                set_node_role_probe_failed,
                writer_ready_reset,
            },
        );

        spawn_ping_loop(status, tx.clone());

        Self {
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
            node_role_probe_failed,
            set_node_role_probe_failed,
            msg_seq,
            connection_epoch,
            writer_session_nonce: new_writer_session_nonce(),
            msg_queue,
            tx,
            #[cfg(test)]
            test_rx: None,
        }
    }

    pub fn send(&self, msg: ClientMessage) {
        if let Err(e) = self.tx.unbounded_send(msg) {
            leptos::logging::error!("消息入队失败: {:?}", e);
        }
    }

    pub fn messages_since(&self, after_seq: u64) -> Vec<(u64, u64, ServerMessage)> {
        self.msg_queue
            .get_untracked()
            .into_iter()
            .filter(|(seq, _, _)| *seq > after_seq)
            .collect()
    }
}

pub(crate) use self::readiness::is_current_connection_message;

#[cfg(test)]
mod tests;

//! plan_ref:
//!   - 09_auth#unauthorized-handling
//!   - 09_auth#unauthorized-disconnected-ui
//!

use deve_core::protocol::{ClientMessage, ServerMessage};
use futures::channel::mpsc::unbounded;
use leptos::prelude::{Set, signal};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::{ConnectionStatus, WsService};

impl WsService {
    pub(crate) fn new_for_test(status: ConnectionStatus) -> Self {
        Self::new_with_incoming_for_test(status, 0, VecDeque::new())
    }

    pub(crate) fn new_with_incoming_for_test(
        status: ConnectionStatus,
        current_connection_epoch: u64,
        messages: VecDeque<(u64, u64, ServerMessage)>,
    ) -> Self {
        let (status, set_status) = signal(status);
        let (writer_ready_repo_id, set_writer_ready_repo_id) = signal(None::<String>);
        let (writer_ready_scope_nonce, set_writer_ready_scope_nonce) = signal(None::<u64>);
        let (writer_client_id, set_writer_client_id) = signal(None::<u64>);
        let msg_seq_value = messages.back().map_or(0, |(seq, _, _)| *seq);
        let (msg_seq, _set_msg_seq) = signal(msg_seq_value);
        let (connection_epoch, _set_connection_epoch) = signal(current_connection_epoch);
        let (msg_queue, _set_msg_queue) = signal(messages);
        let (endpoint, _set_endpoint) = signal(String::new());
        let (node_role, set_node_role) = signal(String::new());
        let (node_role_probe_failed, set_node_role_probe_failed) = signal(false);
        let (tx, rx) = unbounded::<ClientMessage>();

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
            node_role_probe_failed,
            set_node_role_probe_failed,
            msg_seq,
            connection_epoch,
            msg_queue,
            tx,
            test_rx: Some(Arc::new(Mutex::new(rx))),
        }
    }

    pub(crate) fn set_node_role_for_test(&self, node_role: impl Into<String>) {
        self.set_node_role.set(node_role.into());
        self.set_node_role_probe_failed.set(false);
    }

    pub(crate) fn set_node_role_probe_failed_for_test(&self) {
        self.set_node_role.set(String::new());
        self.set_node_role_probe_failed.set(true);
    }

    pub(crate) fn drain_sent_for_test(&self) -> Vec<ClientMessage> {
        let Some(test_rx) = &self.test_rx else {
            return Vec::new();
        };
        let mut rx = test_rx.lock().expect("test receiver lock");
        let mut messages = Vec::new();
        while let Ok(message) = rx.try_recv() {
            messages.push(message);
        }
        messages
    }
}

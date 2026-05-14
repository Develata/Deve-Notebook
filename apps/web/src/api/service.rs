//! plan_ref:
//!   - 09_auth#unauthorized-handling
//!   - 09_auth#unauthorized-disconnected-ui
//!

use deve_core::native_adapter::NativeRuntimeReadiness;
use deve_core::protocol::{ClientMessage, ServerMessage};
#[cfg(test)]
use futures::channel::mpsc::UnboundedReceiver;
use futures::channel::mpsc::{UnboundedSender, unbounded};
use leptos::prelude::*;
use std::collections::VecDeque;
#[cfg(test)]
use std::sync::{Arc, Mutex};

use self::readiness::{
    NativeRuntimeConnectionState, NativeRuntimeReadinessTarget,
    native_runtime_readiness_from_parts, writer_ready_matches,
};
use self::service_ping::spawn_ping_loop;
use super::connection::{ConnectionLifecycle, ConnectionManagerSignals, spawn_connection_manager};
use super::status::ConnectionStatus;
use super::writer_id::{derive_writer_client_id, new_writer_session_nonce};

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
        let (node_role_probe_failed, set_node_role_probe_failed) = signal(false);
        let (tx, rx) = unbounded::<ClientMessage>();
        let lifecycle = ConnectionLifecycle::new();
        let cleanup_lifecycle = lifecycle.clone();
        on_cleanup(move || cleanup_lifecycle.shutdown());

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
                set_node_role_probe_failed,
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

    pub fn mark_unauthorized(&self) {
        self.clear_writer_ready();
        self.set_status.set(ConnectionStatus::Unauthorized);
    }

    pub fn mark_writer_ready(&self, repo_id: impl Into<String>, scope_nonce: u64, peer_id: &str) {
        self.set_writer_ready_repo_id.set(Some(repo_id.into()));
        self.set_writer_ready_scope_nonce.set(Some(scope_nonce));
        self.set_writer_client_id.set(Some(derive_writer_client_id(
            peer_id,
            self.writer_session_nonce,
        )));
    }

    pub fn clear_writer_ready(&self) {
        self.set_writer_ready_repo_id.set(None);
        self.set_writer_ready_scope_nonce.set(None);
        self.set_writer_client_id.set(None);
    }

    pub(crate) fn begin_foreground_reprobe(&self) {
        self.clear_writer_ready();
        self.set_node_role.set(String::new());
        self.set_node_role_probe_failed.set(true);
    }

    pub(crate) fn complete_foreground_node_role_reprobe(&self, summary: impl Into<String>) {
        self.set_node_role.set(summary.into());
        self.set_node_role_probe_failed.set(false);
    }

    pub(crate) fn fail_foreground_node_role_reprobe(&self) {
        self.set_node_role.set(String::new());
        self.set_node_role_probe_failed.set(true);
    }

    pub fn writer_ready_for(&self, repo_id: Option<&str>, scope_nonce: Option<u64>) -> bool {
        let ready_repo_id = self.writer_ready_repo_id.get_untracked();
        writer_ready_matches(
            ready_repo_id.as_deref(),
            self.writer_ready_scope_nonce.get_untracked(),
            repo_id,
            scope_nonce,
        )
    }

    pub fn native_runtime_readiness_for(
        &self,
        repo_id: Option<&str>,
        scope_nonce: Option<u64>,
        handshake_ready: bool,
    ) -> NativeRuntimeReadiness {
        native_runtime_readiness_from_parts(
            NativeRuntimeConnectionState {
                status: self.status.get(),
                node_role: self.node_role.get(),
                node_role_probe_failed: self.node_role_probe_failed.get(),
                ready_repo_id: self.writer_ready_repo_id.get(),
                ready_scope_nonce: self.writer_ready_scope_nonce.get(),
            },
            NativeRuntimeReadinessTarget {
                repo_id,
                scope_nonce,
                handshake_ready,
            },
        )
    }

    pub fn native_runtime_readiness_for_untracked(
        &self,
        repo_id: Option<&str>,
        scope_nonce: Option<u64>,
        handshake_ready: bool,
    ) -> NativeRuntimeReadiness {
        native_runtime_readiness_from_parts(
            NativeRuntimeConnectionState {
                status: self.status.get_untracked(),
                node_role: self.node_role.get_untracked(),
                node_role_probe_failed: self.node_role_probe_failed.get_untracked(),
                ready_repo_id: self.writer_ready_repo_id.get_untracked(),
                ready_scope_nonce: self.writer_ready_scope_nonce.get_untracked(),
            },
            NativeRuntimeReadinessTarget {
                repo_id,
                scope_nonce,
                handshake_ready,
            },
        )
    }

    pub fn writer_client_id_for(
        &self,
        repo_id: Option<&str>,
        scope_nonce: Option<u64>,
    ) -> Option<u64> {
        match (self.writer_client_id.get_untracked(), repo_id, scope_nonce) {
            (Some(client_id), Some(repo_id), Some(scope_nonce))
                if self.writer_ready_for(Some(repo_id), Some(scope_nonce)) =>
            {
                Some(client_id)
            }
            _ => None,
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

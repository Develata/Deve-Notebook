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

use self::service_ping::spawn_ping_loop;
use super::connection::spawn_connection_manager;
use super::status::ConnectionStatus;
use super::writer_id::derive_writer_client_id;

mod service_ping;

#[allow(dead_code)]
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
    pub msg_seq: ReadSignal<u64>,
    pub connection_epoch: ReadSignal<u64>,
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
        let (tx, rx) = unbounded::<ClientMessage>();

        spawn_connection_manager(
            rx,
            set_status,
            set_msg_seq,
            set_msg_queue,
            set_connection_epoch,
            set_endpoint,
            set_node_role,
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
            msg_seq,
            connection_epoch,
            msg_queue,
            tx,
            #[cfg(test)]
            test_rx: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(status: ConnectionStatus) -> Self {
        Self::new_with_incoming_for_test(status, 0, VecDeque::new())
    }

    #[cfg(test)]
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
            msg_seq,
            connection_epoch,
            msg_queue,
            tx,
            test_rx: Some(Arc::new(Mutex::new(rx))),
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
        self.set_writer_client_id
            .set(Some(derive_writer_client_id(peer_id)));
    }

    pub fn clear_writer_ready(&self) {
        self.set_writer_ready_repo_id.set(None);
        self.set_writer_ready_scope_nonce.set(None);
        self.set_writer_client_id.set(None);
    }

    #[cfg(test)]
    pub(crate) fn set_node_role_for_test(&self, node_role: impl Into<String>) {
        self.set_node_role.set(node_role.into());
    }

    pub fn writer_ready_for(&self, repo_id: Option<&str>, scope_nonce: Option<u64>) -> bool {
        writer_ready_matches(
            self.writer_ready_repo_id.get_untracked(),
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
            self.status.get(),
            self.node_role.get(),
            self.writer_ready_repo_id.get(),
            self.writer_ready_scope_nonce.get(),
            repo_id,
            scope_nonce,
            handshake_ready,
        )
    }

    pub fn native_runtime_readiness_for_untracked(
        &self,
        repo_id: Option<&str>,
        scope_nonce: Option<u64>,
        handshake_ready: bool,
    ) -> NativeRuntimeReadiness {
        native_runtime_readiness_from_parts(
            self.status.get_untracked(),
            self.node_role.get_untracked(),
            self.writer_ready_repo_id.get_untracked(),
            self.writer_ready_scope_nonce.get_untracked(),
            repo_id,
            scope_nonce,
            handshake_ready,
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

    #[cfg(test)]
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

pub(crate) fn is_current_connection_message(message_epoch: u64, current_epoch: u64) -> bool {
    message_epoch == current_epoch
}

fn writer_ready_matches(
    ready_repo_id: Option<String>,
    ready_scope_nonce: Option<u64>,
    repo_id: Option<&str>,
    scope_nonce: Option<u64>,
) -> bool {
    match (ready_repo_id, ready_scope_nonce, repo_id, scope_nonce) {
        (Some(ready_repo_id), Some(ready_scope_nonce), Some(repo_id), Some(scope_nonce)) => {
            ready_repo_id == repo_id && ready_scope_nonce == scope_nonce
        }
        _ => false,
    }
}

fn native_runtime_readiness_from_parts(
    status: ConnectionStatus,
    node_role: String,
    ready_repo_id: Option<String>,
    ready_scope_nonce: Option<u64>,
    repo_id: Option<&str>,
    scope_nonce: Option<u64>,
    handshake_ready: bool,
) -> NativeRuntimeReadiness {
    NativeRuntimeReadiness {
        endpoint_reachable: status == ConnectionStatus::Connected,
        auth_status_valid: !matches!(
            status,
            ConnectionStatus::Unauthorized
                | ConnectionStatus::NativeBootstrapInvalid
                | ConnectionStatus::NativeSessionPending
        ),
        node_role_readable: !node_role.trim().is_empty(),
        repo_handshake_complete: handshake_ready,
        writer_ready: writer_ready_matches(ready_repo_id, ready_scope_nonce, repo_id, scope_nonce),
        scope_nonce_current: matches!(
            (ready_scope_nonce, scope_nonce),
            (Some(ready_scope_nonce), Some(scope_nonce)) if ready_scope_nonce == scope_nonce
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{ConnectionStatus, WsService, is_current_connection_message, writer_ready_matches};

    #[test]
    fn dashboard_metrics_stale_connection_epoch_is_not_current() {
        assert!(is_current_connection_message(3, 3));
        assert!(!is_current_connection_message(2, 3));
    }

    #[test]
    fn writer_ready_requires_matching_repo_and_scope_nonce() {
        assert!(writer_ready_matches(
            Some("repo-a".into()),
            Some(7),
            Some("repo-a"),
            Some(7),
        ));
        assert!(!writer_ready_matches(
            Some("repo-a".into()),
            Some(7),
            Some("repo-a"),
            Some(8),
        ));
        assert!(!writer_ready_matches(
            Some("repo-a".into()),
            Some(7),
            Some("repo-b"),
            Some(7),
        ));
        assert!(!writer_ready_matches(
            Some("repo-a".into()),
            Some(7),
            Some("repo-a"),
            None,
        ));
    }

    #[test]
    fn native_runtime_readiness_requires_node_role_writer_and_current_scope() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);

        let missing_node_role =
            ws.native_runtime_readiness_for_untracked(Some("repo-a"), Some(7), true);
        assert!(!missing_node_role.node_role_readable);
        assert!(!missing_node_role.is_runtime_ready());

        ws.set_node_role_for_test("main");
        ws.mark_writer_ready("repo-a", 7, "web-light-peer");
        let ready = ws.native_runtime_readiness_for_untracked(Some("repo-a"), Some(7), true);
        assert!(ready.is_runtime_ready());

        let wrong_repo = ws.native_runtime_readiness_for_untracked(Some("repo-b"), Some(7), true);
        assert!(!wrong_repo.writer_ready);
        assert!(!wrong_repo.is_runtime_ready());

        let stale_scope = ws.native_runtime_readiness_for_untracked(Some("repo-a"), Some(8), true);
        assert!(!stale_scope.scope_nonce_current);
        assert!(!stale_scope.writer_ready);
        assert!(!stale_scope.is_runtime_ready());
    }
}

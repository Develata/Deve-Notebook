//! plan_ref:
//!   - 09_auth#unauthorized-handling
//!   - 09_auth#unauthorized-disconnected-ui
//!

use deve_core::protocol::{ClientMessage, ServerMessage};
use futures::channel::mpsc::{UnboundedSender, unbounded};
use leptos::prelude::*;
use std::collections::VecDeque;

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
    pub msg_seq: ReadSignal<u64>,
    msg_queue: ReadSignal<VecDeque<(u64, ServerMessage)>>,
    tx: UnboundedSender<ClientMessage>,
}

impl WsService {
    pub fn new() -> Self {
        let (status, set_status) = signal(ConnectionStatus::Disconnected);
        let (writer_ready_repo_id, set_writer_ready_repo_id) = signal(None::<String>);
        let (writer_ready_scope_nonce, set_writer_ready_scope_nonce) = signal(None::<u64>);
        let (writer_client_id, set_writer_client_id) = signal(None::<u64>);
        let (msg_seq, set_msg_seq) = signal(0u64);
        let (msg_queue, set_msg_queue) = signal(VecDeque::<(u64, ServerMessage)>::new());
        let (endpoint, set_endpoint) = signal(String::new());
        let (node_role, set_node_role) = signal(String::new());
        let (tx, rx) = unbounded::<ClientMessage>();

        spawn_connection_manager(
            rx,
            set_status,
            set_msg_seq,
            set_msg_queue,
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
            msg_seq,
            msg_queue,
            tx,
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

    pub fn writer_ready_for(&self, repo_id: Option<&str>, scope_nonce: Option<u64>) -> bool {
        writer_ready_matches(
            self.writer_ready_repo_id.get_untracked(),
            self.writer_ready_scope_nonce.get_untracked(),
            repo_id,
            scope_nonce,
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

    pub fn messages_since(&self, after_seq: u64) -> Vec<(u64, ServerMessage)> {
        self.msg_queue
            .get_untracked()
            .into_iter()
            .filter(|(seq, _)| *seq > after_seq)
            .collect()
    }
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

#[cfg(test)]
mod tests {
    use super::writer_ready_matches;

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
}

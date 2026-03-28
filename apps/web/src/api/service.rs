use deve_core::protocol::{ClientMessage, ServerMessage};
use futures::channel::mpsc::{UnboundedSender, unbounded};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::VecDeque;

use super::connection::spawn_connection_manager;
use super::status::ConnectionStatus;
use super::writer_id::derive_writer_client_id;

#[allow(dead_code)]
#[derive(Clone)]
pub struct WsService {
    pub status: ReadSignal<ConnectionStatus>,
    set_status: WriteSignal<ConnectionStatus>,
    pub writer_ready_repo_id: ReadSignal<Option<String>>,
    set_writer_ready_repo_id: WriteSignal<Option<String>>,
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

        let tx_clone = tx.clone();
        let status_check = status;
        spawn_local(async move {
            loop {
                gloo_timers::future::TimeoutFuture::new(30_000).await;
                if status_check.get_untracked() == ConnectionStatus::Connected {
                    let _ = tx_clone.unbounded_send(ClientMessage::Ping);
                }
            }
        });

        Self {
            status,
            set_status,
            writer_ready_repo_id,
            set_writer_ready_repo_id,
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

    pub fn mark_writer_ready(&self, repo_id: impl Into<String>, peer_id: &str) {
        self.set_writer_ready_repo_id.set(Some(repo_id.into()));
        self.set_writer_client_id
            .set(Some(derive_writer_client_id(peer_id)));
    }

    pub fn clear_writer_ready(&self) {
        self.set_writer_ready_repo_id.set(None);
        self.set_writer_client_id.set(None);
    }

    pub fn writer_ready_for(&self, repo_id: Option<&str>) -> bool {
        match (self.writer_ready_repo_id.get_untracked(), repo_id) {
            (Some(ready_repo_id), Some(repo_id)) => ready_repo_id == repo_id,
            _ => false,
        }
    }

    pub fn writer_client_id_for(&self, repo_id: Option<&str>) -> Option<u64> {
        match (
            self.writer_ready_repo_id.get_untracked(),
            self.writer_client_id.get_untracked(),
            repo_id,
        ) {
            (Some(ready_repo_id), Some(client_id), Some(repo_id)) if ready_repo_id == repo_id => {
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

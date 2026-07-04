//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 08_auth#unauthorized-disconnected-ui
//!

use super::super::ConnectionStatus;
use super::super::incoming::handle_socket_event;
use super::super::output::{is_write_message, send_or_requeue};
use super::super::socket::{BrowserSocket, SocketEvent};
use super::super::write_gate::{WriterReadyResetSignals, set_status_and_revoke_writer_ready};
use super::ConnectionLifecycle;
use futures::FutureExt;
use futures::StreamExt;
use futures::channel::mpsc::UnboundedReceiver;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use std::collections::VecDeque;

#[derive(Clone)]
pub(super) struct ConnectedSessionSignals {
    pub lifecycle: ConnectionLifecycle,
    pub set_status: WriteSignal<ConnectionStatus>,
    pub set_msg_seq: WriteSignal<u64>,
    pub set_msg_queue: WriteSignal<VecDeque<(u64, u64, deve_core::protocol::ServerMessage)>>,
    pub set_node_role: WriteSignal<String>,
    pub set_source_control_authority: WriteSignal<String>,
    pub set_host_file_copy_absolute_path: WriteSignal<bool>,
    pub set_host_file_reveal_in_system_explorer: WriteSignal<bool>,
    pub set_node_role_probe_failed: WriteSignal<bool>,
    pub writer_ready_reset: WriterReadyResetSignals,
    pub connection_epoch: u64,
}

pub(super) async fn run_connected_session(
    socket: BrowserSocket,
    mut events: UnboundedReceiver<SocketEvent>,
    rx: &mut UnboundedReceiver<deve_core::protocol::ClientMessage>,
    queue: &mut VecDeque<deve_core::protocol::ClientMessage>,
    signals: ConnectedSessionSignals,
) {
    let mut confirmed_connected = false;
    let mut announced_open = false;

    loop {
        if !signals.lifecycle.is_active() {
            return;
        }
        if browser_reports_offline() {
            leptos::logging::warn!("WS session ended because browser reports offline");
            let _ = try_set_session_status(&signals, ConnectionStatus::Disconnected);
            return;
        }
        if socket.is_open() && !announced_open {
            leptos::logging::log!("WS: Socket opened, waiting for first message...");
            announced_open = true;
        }

        if socket.is_closed() {
            leptos::logging::warn!(
                "WS session ended because browser socket is closed: ready_state={}",
                socket.ready_state()
            );
            return;
        }

        if socket.is_open()
            && let Some(msg) = queue.pop_front()
            && !send_or_requeue(&socket, msg, queue)
        {
            return;
        }

        let inbound = events.next().fuse();
        let outbound = rx.next().fuse();
        let timer = TimeoutFuture::new(25).fuse();
        futures::pin_mut!(inbound, outbound, timer);

        futures::select! {
            result = inbound => match result {
                Some(event) => {
                    if matches!(event, SocketEvent::Opened) && !announced_open && socket.is_open() {
                        leptos::logging::log!("WS: Socket opened, waiting for first message...");
                        announced_open = true;
                    }
                    if !handle_socket_event(
                        event,
                        &mut confirmed_connected,
                        signals.set_msg_seq,
                        signals.set_msg_queue,
                        signals.set_status,
                        signals.connection_epoch,
                    ) {
                        return;
                    }
                }
                None => {
                    if socket.is_closed() {
                        return;
                    }
                }
            },
            maybe_msg = outbound => match maybe_msg {
                Some(msg) => {
                    if !confirmed_connected && is_write_message(&msg) {
                        leptos::logging::warn!("WebLightPeer: 断连时禁止写入消息 {:?}", msg);
                        continue;
                    }
                    queue.push_back(msg);
                }
                None => return,
            },
            _ = timer => {}
        }
    }
}

fn try_set_session_status(signals: &ConnectedSessionSignals, status: ConnectionStatus) -> bool {
    if !signals.lifecycle.is_active() {
        return false;
    }
    if status != ConnectionStatus::Connected && !reset_node_role_runtime(signals) {
        return false;
    }
    set_status_and_revoke_writer_ready(signals.set_status, signals.writer_ready_reset, status)
}

fn reset_node_role_runtime(signals: &ConnectedSessionSignals) -> bool {
    signals
        .lifecycle
        .try_set(signals.set_node_role, String::new())
        && signals
            .lifecycle
            .try_set(signals.set_source_control_authority, "unknown".to_string())
        && signals
            .lifecycle
            .try_set(signals.set_host_file_copy_absolute_path, false)
        && signals
            .lifecycle
            .try_set(signals.set_host_file_reveal_in_system_explorer, false)
        && signals
            .lifecycle
            .try_set(signals.set_node_role_probe_failed, false)
}

#[cfg(target_arch = "wasm32")]
fn browser_reports_offline() -> bool {
    web_sys::window()
        .map(|window| !window.navigator().on_line())
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_reports_offline() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::write_gate::WriterReadyResetSignals;
    use leptos::prelude::GetUntracked;

    #[test]
    fn disconnected_session_status_resets_node_role_runtime_summary() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (status, set_status) = signal(ConnectionStatus::Connected);
        let (_msg_seq, set_msg_seq) = signal(0u64);
        let (_msg_queue, set_msg_queue) =
            signal(VecDeque::<(u64, u64, deve_core::protocol::ServerMessage)>::new());
        let (node_role, set_node_role) = signal("main".to_string());
        let (source_control_authority, set_source_control_authority) = signal("mirror".to_string());
        let (host_file_copy_absolute_path, set_host_file_copy_absolute_path) = signal(true);
        let (host_file_reveal_in_system_explorer, set_host_file_reveal_in_system_explorer) =
            signal(true);
        let (node_role_probe_failed, set_node_role_probe_failed) = signal(true);
        let (writer_ready_repo_id, set_writer_ready_repo_id) = signal(Some("repo-a".to_string()));
        let (writer_ready_scope_nonce, set_writer_ready_scope_nonce) = signal(Some(7u64));
        let (writer_client_id, set_writer_client_id) = signal(Some(9u64));

        let signals = ConnectedSessionSignals {
            lifecycle: ConnectionLifecycle::new(),
            set_status,
            set_msg_seq,
            set_msg_queue,
            set_node_role,
            set_source_control_authority,
            set_host_file_copy_absolute_path,
            set_host_file_reveal_in_system_explorer,
            set_node_role_probe_failed,
            writer_ready_reset: WriterReadyResetSignals::new(
                set_writer_ready_repo_id,
                set_writer_ready_scope_nonce,
                set_writer_client_id,
            ),
            connection_epoch: 1,
        };

        assert!(try_set_session_status(
            &signals,
            ConnectionStatus::Disconnected
        ));

        assert_eq!(status.get_untracked(), ConnectionStatus::Disconnected);
        assert_eq!(node_role.get_untracked(), "");
        assert_eq!(source_control_authority.get_untracked(), "unknown");
        assert!(!host_file_copy_absolute_path.get_untracked());
        assert!(!host_file_reveal_in_system_explorer.get_untracked());
        assert!(!node_role_probe_failed.get_untracked());
        assert!(writer_ready_repo_id.get_untracked().is_none());
        assert!(writer_ready_scope_nonce.get_untracked().is_none());
        assert!(writer_client_id.get_untracked().is_none());
    }
}

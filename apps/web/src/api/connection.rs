// apps\web\src\api
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!   - 08_auth#unauthorized-handling
//!   - 08_auth#unauthorized-disconnected-ui
//!
//! # WebSocket 连接管理器
//!
//! ## 职责
//! 1. 建立 WebSocket 连接
//! 2. 指数退避重连策略
//! 3. 读取服务器消息并更新信号
//! 4. 将连接会话委托给 `connection_session`

mod lifecycle;
mod session;

use futures::channel::mpsc::UnboundedReceiver;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::VecDeque;

pub(super) use self::lifecycle::ConnectionLifecycle;
use self::session::ConnectedSessionSignals;
use super::ConnectionStatus;
use super::auth_probe::{AuthProbe, probe_auth_status_with_http_base};
use super::backoff::BackoffStrategy;
use super::connection_role::{fetch_node_role, fetch_node_role_for_http_base};
use super::connection_urls::{build_same_origin_ws_url, build_ws_urls_for_native_state};
use super::native_bootstrap::read_native_bootstrap;
use super::native_http::preferred_http_base;
use super::output::prepare_queue_for_new_connection;
use super::socket::BrowserSocket;
use super::write_gate::{WriterReadyResetSignals, set_status_and_revoke_writer_ready};

/// 本地开发后端默认端口；仅在 debug 构建中作为兜底候选。
pub(super) const DEV_WS_PORT: u16 = 3001;

#[derive(Clone)]
pub(super) struct ConnectionManagerSignals {
    pub lifecycle: ConnectionLifecycle,
    pub set_status: WriteSignal<ConnectionStatus>,
    pub set_msg_seq: WriteSignal<u64>,
    pub set_msg_queue: WriteSignal<VecDeque<(u64, u64, deve_core::protocol::ServerMessage)>>,
    pub current_connection_epoch: ReadSignal<u64>,
    pub set_connection_epoch: WriteSignal<u64>,
    pub set_endpoint: WriteSignal<String>,
    pub set_node_role: WriteSignal<String>,
    pub set_source_control_git_bridge: WriteSignal<String>,
    pub set_node_role_probe_failed: WriteSignal<bool>,
    pub writer_ready_reset: WriterReadyResetSignals,
}

/// 启动连接管理器任务
pub fn spawn_connection_manager(
    mut rx: UnboundedReceiver<deve_core::protocol::ClientMessage>,
    signals: ConnectionManagerSignals,
) {
    spawn_local(async move {
        let native_bootstrap = read_native_bootstrap();
        let auth_http_base = preferred_http_base();
        if let Some(blocked_status) = native_bootstrap.blocked_status() {
            leptos::logging::error!("Native bootstrap is present but not ready; refusing fallback");
            let _ = try_set_connection_status(&signals, blocked_status);
            return;
        }

        let urls = build_ws_urls_for_native_state(&native_bootstrap);
        let mut url_idx = 0usize;
        let mut backoff = BackoffStrategy::new();
        let mut queue = VecDeque::new();
        let mut connection_epoch = 0u64;

        loop {
            if !signals.lifecycle.is_active() {
                return;
            }
            let url = urls
                .get(url_idx)
                .cloned()
                .unwrap_or_else(build_same_origin_ws_url);
            connection_epoch = connection_epoch.saturating_add(1);
            if !signals
                .lifecycle
                .try_set(signals.set_connection_epoch, connection_epoch)
            {
                return;
            }
            if !try_set_connection_status(&signals, ConnectionStatus::Connecting) {
                return;
            }
            leptos::logging::log!("WS: Connecting to {}...", url);

            match BrowserSocket::connect(&url) {
                Ok((socket, events)) => {
                    if !signals.lifecycle.try_set(signals.set_endpoint, url.clone())
                        || !signals
                            .lifecycle
                            .try_set(signals.set_node_role, String::new())
                        || !signals
                            .lifecycle
                            .try_set(signals.set_source_control_git_bridge, "unknown".to_string())
                        || !signals
                            .lifecycle
                            .try_set(signals.set_node_role_probe_failed, false)
                    {
                        return;
                    }
                    if let Some(http_base) = auth_http_base.clone() {
                        spawn_local(fetch_node_role_for_http_base(
                            signals.lifecycle.clone(),
                            http_base,
                            signals.set_node_role,
                            signals.set_source_control_git_bridge,
                            signals.set_node_role_probe_failed,
                            signals.current_connection_epoch,
                            connection_epoch,
                        ));
                    } else {
                        spawn_local(fetch_node_role(
                            signals.lifecycle.clone(),
                            url.clone(),
                            signals.set_node_role,
                            signals.set_source_control_git_bridge,
                            signals.set_node_role_probe_failed,
                            signals.current_connection_epoch,
                            connection_epoch,
                        ));
                    }
                    backoff.reset();
                    prepare_queue_for_new_connection(&mut queue);
                    session::run_connected_session(
                        socket,
                        events,
                        &mut rx,
                        &mut queue,
                        ConnectedSessionSignals {
                            lifecycle: signals.lifecycle.clone(),
                            set_status: signals.set_status,
                            set_msg_seq: signals.set_msg_seq,
                            set_msg_queue: signals.set_msg_queue,
                            set_node_role: signals.set_node_role,
                            set_source_control_git_bridge: signals.set_source_control_git_bridge,
                            set_node_role_probe_failed: signals.set_node_role_probe_failed,
                            writer_ready_reset: signals.writer_ready_reset,
                            connection_epoch,
                        },
                    )
                    .await;

                    if !signals.lifecycle.is_active() {
                        return;
                    }
                    leptos::logging::log!("WS: Connection Lost");

                    if matches!(
                        probe_auth_status_with_http_base(auth_http_base.as_deref()).await,
                        AuthProbe::Invalid
                    ) {
                        let _ = try_set_connection_status(&signals, ConnectionStatus::Unauthorized);
                        return;
                    }
                }
                Err(e) => {
                    leptos::logging::error!("WS Open Error: {:?}", e);
                    if !signals.lifecycle.is_active() {
                        return;
                    }
                    if matches!(
                        probe_auth_status_with_http_base(auth_http_base.as_deref()).await,
                        AuthProbe::Invalid
                    ) {
                        let _ = try_set_connection_status(&signals, ConnectionStatus::Unauthorized);
                        return;
                    }
                    if url_idx + 1 < urls.len() {
                        url_idx += 1;
                        continue;
                    }
                }
            }

            if !signals.lifecycle.is_active() {
                return;
            }
            if matches!(
                probe_auth_status_with_http_base(auth_http_base.as_deref()).await,
                AuthProbe::Invalid
            ) {
                let _ = try_set_connection_status(&signals, ConnectionStatus::Unauthorized);
                return;
            }
            if !try_set_connection_status(&signals, ConnectionStatus::Disconnected) {
                return;
            }
            backoff.wait().await;
            url_idx = 0;
        }
    });
}

fn try_set_connection_status(signals: &ConnectionManagerSignals, status: ConnectionStatus) -> bool {
    if !signals.lifecycle.is_active() {
        return false;
    }
    if status != ConnectionStatus::Connected && !reset_node_role_runtime(signals) {
        return false;
    }
    set_status_and_revoke_writer_ready(signals.set_status, signals.writer_ready_reset, status)
}

fn reset_node_role_runtime(signals: &ConnectionManagerSignals) -> bool {
    signals
        .lifecycle
        .try_set(signals.set_node_role, String::new())
        && signals
            .lifecycle
            .try_set(signals.set_source_control_git_bridge, "unknown".to_string())
        && signals
            .lifecycle
            .try_set(signals.set_node_role_probe_failed, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::write_gate::WriterReadyResetSignals;
    use leptos::prelude::GetUntracked;
    use std::collections::VecDeque;

    #[test]
    fn non_connected_status_resets_node_role_runtime_summary() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (status, set_status) = signal(ConnectionStatus::Connected);
        let (msg_seq, set_msg_seq) = signal(0u64);
        let (_msg_queue, set_msg_queue) =
            signal(VecDeque::<(u64, u64, deve_core::protocol::ServerMessage)>::new());
        let (connection_epoch, set_connection_epoch) = signal(1u64);
        let (_endpoint, set_endpoint) = signal("ws://127.0.0.1:3001/ws".to_string());
        let (node_role, set_node_role) = signal("main".to_string());
        let (source_control_git_bridge, set_source_control_git_bridge) =
            signal("mirror".to_string());
        let (node_role_probe_failed, set_node_role_probe_failed) = signal(true);
        let (writer_ready_repo_id, set_writer_ready_repo_id) = signal(Some("repo-a".to_string()));
        let (writer_ready_scope_nonce, set_writer_ready_scope_nonce) = signal(Some(7u64));
        let (writer_client_id, set_writer_client_id) = signal(Some(9u64));

        let signals = ConnectionManagerSignals {
            lifecycle: ConnectionLifecycle::new(),
            set_status,
            set_msg_seq,
            set_msg_queue,
            current_connection_epoch: connection_epoch,
            set_connection_epoch,
            set_endpoint,
            set_node_role,
            set_source_control_git_bridge,
            set_node_role_probe_failed,
            writer_ready_reset: WriterReadyResetSignals::new(
                set_writer_ready_repo_id,
                set_writer_ready_scope_nonce,
                set_writer_client_id,
            ),
        };

        assert!(try_set_connection_status(
            &signals,
            ConnectionStatus::Disconnected
        ));

        assert_eq!(status.get_untracked(), ConnectionStatus::Disconnected);
        assert_eq!(node_role.get_untracked(), "");
        assert_eq!(source_control_git_bridge.get_untracked(), "unknown");
        assert!(!node_role_probe_failed.get_untracked());
        assert!(writer_ready_repo_id.get_untracked().is_none());
        assert!(writer_ready_scope_nonce.get_untracked().is_none());
        assert!(writer_client_id.get_untracked().is_none());
        assert_eq!(msg_seq.get_untracked(), 0);
    }
}

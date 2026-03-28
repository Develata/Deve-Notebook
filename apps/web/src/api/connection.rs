// apps\web\src\api
//! # WebSocket 连接管理器
//!
//! ## 职责
//! 1. 建立 WebSocket 连接
//! 2. 指数退避重连策略
//! 3. 读取服务器消息并更新信号
//! 4. 将连接会话委托给 `connection_session`

#[path = "connection_session.rs"]
mod session;

use futures::channel::mpsc::UnboundedReceiver;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::VecDeque;

use super::ConnectionStatus;
use super::auth_probe::{AuthProbe, probe_auth_status};
use super::backoff::BackoffStrategy;
use super::connection_role::fetch_node_role;
use super::connection_urls::{build_same_origin_ws_url, build_ws_urls};
use super::output::prepare_queue_for_new_connection;
use super::socket::BrowserSocket;

/// 本地开发后端默认端口；仅在 debug 构建中作为兜底候选。
pub(super) const DEV_WS_PORT: u16 = 3001;

/// 启动连接管理器任务
pub fn spawn_connection_manager(
    mut rx: UnboundedReceiver<deve_core::protocol::ClientMessage>,
    set_status: WriteSignal<ConnectionStatus>,
    set_msg_seq: WriteSignal<u64>,
    set_msg_queue: WriteSignal<VecDeque<(u64, deve_core::protocol::ServerMessage)>>,
    set_endpoint: WriteSignal<String>,
    set_node_role: WriteSignal<String>,
) {
    spawn_local(async move {
        let urls = build_ws_urls();
        let mut url_idx = 0usize;
        let mut backoff = BackoffStrategy::new();
        let mut queue = VecDeque::new();

        loop {
            let url = urls
                .get(url_idx)
                .cloned()
                .unwrap_or_else(build_same_origin_ws_url);
            set_status.set(ConnectionStatus::Connecting);
            leptos::logging::log!("WS: Connecting to {}...", url);

            match BrowserSocket::connect(&url) {
                Ok((socket, events)) => {
                    set_endpoint.set(url.clone());
                    spawn_local(fetch_node_role(url.clone(), set_node_role));
                    backoff.reset();
                    prepare_queue_for_new_connection(&mut queue);
                    session::run_connected_session(
                        socket,
                        events,
                        &mut rx,
                        &mut queue,
                        set_status,
                        set_msg_seq,
                        set_msg_queue,
                    )
                    .await;

                    leptos::logging::log!("WS: Connection Lost");

                    if matches!(probe_auth_status().await, AuthProbe::Invalid) {
                        set_status.set(ConnectionStatus::Unauthorized);
                        return;
                    }
                }
                Err(e) => {
                    leptos::logging::error!("WS Open Error: {:?}", e);
                    if matches!(probe_auth_status().await, AuthProbe::Invalid) {
                        set_status.set(ConnectionStatus::Unauthorized);
                        return;
                    }
                    if url_idx + 1 < urls.len() {
                        url_idx += 1;
                        continue;
                    }
                }
            }

            if matches!(probe_auth_status().await, AuthProbe::Invalid) {
                set_status.set(ConnectionStatus::Unauthorized);
                return;
            }
            set_status.set(ConnectionStatus::Disconnected);
            backoff.wait().await;
            url_idx = 0;
        }
    });
}

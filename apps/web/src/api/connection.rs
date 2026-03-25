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
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::VecDeque;

use super::ConnectionStatus;
use super::auth_probe::{AuthProbe, probe_auth_status};
use super::backoff::BackoffStrategy;
use super::output::prepare_queue_for_new_connection;
use super::socket::BrowserSocket;

/// 本地开发后端默认端口；仅在 debug 构建中作为兜底候选。
const DEV_WS_PORT: u16 = 3001;

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

async fn fetch_node_role(ws_url: String, set_node_role: WriteSignal<String>) {
    let http_url = ws_url
        .replace("wss://", "https://")
        .replace("ws://", "http://")
        .replace("/ws", "");
    let url = format!("{}/api/node/role", http_url);
    let res = Request::get(&url).send().await;
    if let Ok(resp) = res
        && let Ok(json) = resp.json::<serde_json::Value>().await
    {
        let role = json
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let main_port = json.get("main_port").and_then(|v| v.as_u64()).unwrap_or(0);
        let ws_port = json.get("ws_port").and_then(|v| v.as_u64()).unwrap_or(0);
        let text = if role == "proxy" && main_port > 0 {
            format!("proxy → {} (ws:{})", main_port, ws_port)
        } else if ws_port > 0 {
            format!("{} (ws:{})", role, ws_port)
        } else {
            role.to_string()
        };
        set_node_role.set(text);
    }
}

/// 根据当前来源构建 WebSocket URL；自动检测 HTTPS 并升级为 wss://。
/// 默认使用同源 WebSocket 路径 (/ws)，由反向代理或 trunk 代理转发
fn build_same_origin_ws_url() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return format!("ws://localhost:{DEV_WS_PORT}/ws"),
    };
    let location = window.location();
    let host = location
        .host()
        .unwrap_or_else(|_| "localhost:3001".to_string());
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let ws_scheme = if protocol == "https:" { "wss" } else { "ws" };
    format!("{}://{}/ws", ws_scheme, host)
}

fn build_ws_urls() -> Vec<String> {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return vec![format!("ws://localhost:{DEV_WS_PORT}/ws")],
    };
    let location = window.location();
    let hostname = normalize_hostname(
        location
            .hostname()
            .unwrap_or_else(|_| "localhost".to_string()),
    );
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let ws_scheme = if protocol == "https:" { "wss" } else { "ws" };
    let mut urls = Vec::new();

    if let Some(port) = query_port() {
        push_ws_url(
            &mut urls,
            format!("{}://{}:{}/ws", ws_scheme, hostname, port),
        );
    }

    push_ws_url(&mut urls, build_same_origin_ws_url());

    if cfg!(debug_assertions) {
        push_ws_url(
            &mut urls,
            format!("{}://{}:{}/ws", ws_scheme, hostname, DEV_WS_PORT),
        );
        push_ws_url(
            &mut urls,
            format!("{}://localhost:{}/ws", ws_scheme, DEV_WS_PORT),
        );
        push_ws_url(
            &mut urls,
            format!("{}://127.0.0.1:{}/ws", ws_scheme, DEV_WS_PORT),
        );
    }

    urls
}

fn normalize_hostname(hostname: String) -> String {
    match hostname.as_str() {
        "" | "0.0.0.0" | "::" | "[::]" => "localhost".to_string(),
        _ => hostname,
    }
}

fn push_ws_url(urls: &mut Vec<String>, url: String) {
    if !urls.iter().any(|current| current == &url) {
        urls.push(url);
    }
}

fn query_port() -> Option<u16> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    if search.is_empty() {
        return None;
    }
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    let val = params.get("ws_port")?;
    val.parse::<u16>().ok()
}

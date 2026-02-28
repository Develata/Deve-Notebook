// apps\web\src\api
//! # WebSocket 连接管理器
//!
//! ## 职责
//! 1. 建立 WebSocket 连接
//! 2. 指数退避重连策略
//! 3. 读取服务器消息并更新信号
//! 4. 通过 link_tx 将写入端传递给输出管理器

use deve_core::protocol::ServerMessage;
use futures::StreamExt;
use futures::channel::mpsc::UnboundedSender;
use futures::stream::SplitSink;
use gloo_net::http::Request;
use gloo_net::websocket::{Message, futures::WebSocket};
use leptos::prelude::*;
use leptos::task::spawn_local;

use super::ConnectionStatus;
use super::backoff::BackoffStrategy;

/// 启动连接管理器任务
pub fn spawn_connection_manager(
    set_status: WriteSignal<ConnectionStatus>,
    set_msg: WriteSignal<Option<ServerMessage>>,
    set_endpoint: WriteSignal<String>,
    set_node_role: WriteSignal<String>,
    link_tx: UnboundedSender<SplitSink<WebSocket, Message>>,
) {
    spawn_local(async move {
        let urls = build_ws_urls();
        let mut url_idx = 0usize;
        let mut backoff = BackoffStrategy::new();

        loop {
            let url = urls
                .get(url_idx)
                .cloned()
                .unwrap_or_else(|| build_ws_url(3001));
            set_status.set(ConnectionStatus::Connecting);
            leptos::logging::log!("WS: Connecting to {}...", url);

            match WebSocket::open(&url) {
                Ok(ws) => {
                    set_endpoint.set(url.clone());
                    spawn_local(fetch_node_role(url.clone(), set_node_role));
                    leptos::logging::log!("WS: Socket opened, waiting for first message...");
                    backoff.reset();

                    let (write, read) = ws.split();

                    // Hand over the writer to the Output Manager
                    if let Err(e) = link_tx.unbounded_send(write) {
                        leptos::logging::error!("Failed to send sink to output loop: {:?}", e);
                    }

                    // Block on reading until disconnect
                    // Pass set_status to confirm connection after first successful message
                    process_incoming_messages(read, set_msg, set_status).await;

                    leptos::logging::log!("WS: Connection Lost (Reader ended)");
                }
                Err(e) => {
                    leptos::logging::error!("WS Open Error: {:?}", e);
                    if url_idx + 1 < urls.len() {
                        url_idx += 1;
                        continue;
                    }
                }
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

/// 从 WebSocket 读取消息直到连接关闭。
/// 在收到第一条成功消息后将状态设置为 Connected。
///
/// ## 协议策略
/// - **优先二进制 (Bincode)**: 体积更小，解析更快，零字符串分配。
/// - **降级 JSON**: 向后兼容旧版服务端或调试场景。
async fn process_incoming_messages(
    mut read: futures::stream::SplitStream<WebSocket>,
    set_msg: WriteSignal<Option<ServerMessage>>,
    set_status: WriteSignal<ConnectionStatus>,
) {
    let mut confirmed_connected = false;

    while let Some(result) = read.next().await {
        match result {
            // 优先处理二进制消息 (Bincode)
            Ok(Message::Bytes(bytes)) => {
                if !confirmed_connected {
                    leptos::logging::log!(
                        "WS: First binary message received, connection confirmed!"
                    );
                    set_status.set(ConnectionStatus::Connected);
                    confirmed_connected = true;
                }

                match bincode::deserialize::<ServerMessage>(&bytes) {
                    Ok(server_msg) => set_msg.set(Some(server_msg)),
                    Err(e) => leptos::logging::error!("Bincode Parse Error: {:?}", e),
                }
            }
            // 向后兼容: JSON 文本消息
            Ok(Message::Text(txt)) => {
                if !confirmed_connected {
                    leptos::logging::log!("WS: First text message received, connection confirmed!");
                    set_status.set(ConnectionStatus::Connected);
                    confirmed_connected = true;
                }

                match serde_json::from_str::<ServerMessage>(&txt) {
                    Ok(server_msg) => set_msg.set(Some(server_msg)),
                    Err(e) => leptos::logging::error!("JSON Parse Error: {:?}", e),
                }
            }
            Err(e) => {
                leptos::logging::error!("WS Read Error: {:?}", e);
                break;
            }
        }
    }
}

/// 根据当前主机名和协议构建 WebSocket URL
///
/// 自动检测 HTTPS 并升级为 wss://
fn build_ws_url(port: u16) -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return format!("ws://localhost:{}/ws", port),
    };
    let location = window.location();
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "localhost".to_string());
    let protocol = location
        .protocol()
        .unwrap_or_else(|_| "http:".to_string());
    let ws_scheme = if protocol == "https:" { "wss" } else { "ws" };
    format!("{}://{}:{}/ws", ws_scheme, hostname, port)
}

fn build_ws_urls() -> Vec<String> {
    let mut urls = Vec::new();
    if let Some(port) = query_port() {
        urls.push(build_ws_url(port));
        return urls;
    }
    for p in 3001..=3005 {
        urls.push(build_ws_url(p));
    }
    urls
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

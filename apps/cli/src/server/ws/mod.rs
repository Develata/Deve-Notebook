use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use bincode::Options;
use futures::StreamExt;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ClientMessage;
use deve_core::security::auth::{config::AuthConfig, jwt};

mod route;
pub(crate) mod send;

/// Bincode 消息大小限制 (防止 DoS 攻击)
/// 16 MB 足以处理大型文档快照
const MAX_BINCODE_SIZE: u64 = 16 * 1024 * 1024;
const COOKIE_NAME: &str = "token";

/// HTTP/WebSocket 入口 (含鉴权)。
///
/// 09_auth.md: "WebSocket Auth: 必须在握手阶段验证 Ticket/Token"
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    axum::Extension(config): axum::Extension<Arc<AuthConfig>>,
    req: axum::http::request::Parts,
) -> impl IntoResponse {
    // 提取 Cookie 中的 JWT
    let token = extract_cookie_from_parts(&req);
    let authed = match token {
        Some(ref t) => jwt::validate_token(&config.secret, t, config.token_version).is_ok(),
        None => false,
    };

    // localhost 免密策略
    let is_local = req
        .extensions
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip().is_loopback())
        .unwrap_or(false);

    if !(authed || config.allow_anonymous_localhost && is_local) {
        return (axum::http::StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let peer_id = uuid::Uuid::new_v4().to_string();
    ws.on_upgrade(move |socket| handle_socket(state, socket, peer_id))
        .into_response()
}

/// WebSocket 消息处理器。
///
/// ## 协议策略
/// - **优先二进制 (Bincode)**: 体积更小，解析更快，零字符串分配。
/// - **降级 JSON**: 向后兼容旧版客户端或调试场景。
pub async fn handle_socket(
    state: Arc<AppState>,
    socket: axum::extract::ws::WebSocket,
    peer_id: String,
) {
    let (sender, mut receiver) = socket.split();

    // 为每个连接创建有界单播队列，避免慢客户端导致无界内存增长。
    let (unicast_tx, unicast_rx) = send::new_unicast_channel();

    // 将单播队列写入 WebSocket。
    send::spawn_unicast_sender_task(sender, unicast_rx);

    // 订阅广播并尝试转发到单播队列（带背压/丢弃策略）。
    let broadcast_rx = state.tx.subscribe();
    send::spawn_broadcast_forwarder(broadcast_rx, unicast_tx.clone());

    let ch = DualChannel::new(state.tx.clone(), unicast_tx);

    tracing::info!("Client connected: {}", peer_id);

    let mut session = WsSession::new();

    // Bincode 配置: 带大小限制防止内存耗尽攻击
    let bincode_config = bincode::options().with_limit(MAX_BINCODE_SIZE);

    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                tracing::warn!("WS receive error: {:?}", e);
                break;
            }
        };

        match msg {
            // 优先处理二进制消息 (Bincode)
            axum::extract::ws::Message::Binary(bin) => {
                match bincode_config.deserialize::<ClientMessage>(&bin) {
                    Ok(client_msg) => {
                        route::route_message(&state, &ch, &mut session, client_msg).await
                    }
                    Err(e) => tracing::warn!("Bincode parse error: {:?}, {} bytes", e, bin.len()),
                }
            }
            // 向后兼容: JSON 文本消息
            axum::extract::ws::Message::Text(text) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        route::route_message(&state, &ch, &mut session, client_msg).await
                    }
                    Err(_) => tracing::warn!("Failed to parse client message: {}", text),
                }
            }
            axum::extract::ws::Message::Close(_) => {
                tracing::info!("Client disconnected: {}", peer_id);
                break;
            }
            _ => {}
        }
    }
}

fn extract_cookie_from_parts(parts: &axum::http::request::Parts) -> Option<String> {
    let header = parts.headers.get("cookie")?.to_str().ok()?;
    for pair in header.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix(COOKIE_NAME) {
            let value = value.trim_start_matches('=');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

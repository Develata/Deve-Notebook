use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::StreamExt;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::security::auth::{config::AuthConfig, jwt};

mod filter;
mod receive;
mod route;
pub(crate) mod send;

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
    let token = req
        .headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(crate::server::auth::cookie::extract_token_from_cookie_header);
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
    ws.on_upgrade(move |socket| handle_socket(state, socket, peer_id, authed))
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
    browser_session: bool,
) {
    let (sender, mut receiver) = socket.split();
    let mut session = WsSession::new();
    if browser_session {
        session.mark_browser_session();
    }

    // 为每个连接创建有界单播队列，避免慢客户端导致无界内存增长。
    let (unicast_tx, unicast_rx) = send::new_unicast_channel();

    // 将单播队列写入 WebSocket。
    send::spawn_unicast_sender_task(sender, unicast_rx);

    // 订阅广播并尝试转发到单播队列（带背压/丢弃策略）。
    let broadcast_rx = state.tx.subscribe();
    let broadcast_filter = send::BroadcastFilter::for_session(&session);
    send::spawn_broadcast_forwarder(broadcast_rx, unicast_tx.clone(), broadcast_filter.clone());

    let ch = DualChannel::new(state.tx.clone(), unicast_tx);

    tracing::info!("Client connected: {}", peer_id);

    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                tracing::warn!("WS receive error: {:?}", e);
                break;
            }
        };

        if matches!(
            receive::handle_incoming_message(
                &state,
                &ch,
                &mut session,
                msg,
                &broadcast_filter,
                &peer_id,
            )
            .await,
            receive::SocketFlow::Break
        ) {
            break;
        }
    }
}

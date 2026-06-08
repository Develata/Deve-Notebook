//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 08_auth#unauthorized-handling
//!
//! Authenticated WebSocket upgrade and session runtime entrypoint.

use axum::Json;
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::StreamExt;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::auth::{AuthErrorCode, AuthErrorResponse};
use deve_core::security::auth::config::AuthConfig;

mod auth;
mod filter;
mod receive;
mod route;
pub(crate) mod send;

#[derive(Debug, Clone)]
pub struct WsAdmissionConfig {
    pub p2p_inbound_token_env: Option<String>,
}

impl Default for WsAdmissionConfig {
    fn default() -> Self {
        Self {
            p2p_inbound_token_env: Some(auth::P2P_INBOUND_TOKEN_ENV.into()),
        }
    }
}

impl WsAdmissionConfig {
    pub fn new(p2p_inbound_token_env: Option<String>) -> Self {
        Self {
            p2p_inbound_token_env,
        }
    }
}

/// HTTP/WebSocket 入口 (含鉴权)。
///
/// 09_auth.md: "WebSocket Auth: 必须在握手阶段验证 Ticket/Token"
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    axum::Extension(config): axum::Extension<Arc<AuthConfig>>,
    axum::Extension(admission_config): axum::Extension<Arc<WsAdmissionConfig>>,
    req: axum::http::request::Parts,
) -> impl IntoResponse {
    let admission = match auth::session_admission(
        &config,
        &req,
        admission_config.p2p_inbound_token_env.as_deref(),
    ) {
        Ok(admission) => admission,
        Err(code) => return unauthorized_ws_response(code),
    };

    let peer_id = uuid::Uuid::new_v4().to_string();
    ws.on_upgrade(move |socket| handle_socket(state, socket, peer_id, admission.is_browser()))
        .into_response()
}

fn unauthorized_ws_response(code: AuthErrorCode) -> axum::response::Response {
    (
        axum::http::StatusCode::UNAUTHORIZED,
        Json(AuthErrorResponse::new(code)),
    )
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

#[cfg(test)]
mod tests {
    use super::auth::is_browser_session_connection;
    use super::unauthorized_ws_response;
    use axum::body;
    use axum::http::StatusCode;
    use deve_core::protocol::auth::{AuthErrorCode, AuthErrorResponse};

    #[test]
    fn localhost_anonymous_ws_still_counts_as_browser_session() {
        assert!(is_browser_session_connection(false, true, true));
    }

    #[test]
    fn remote_anonymous_ws_is_rejected() {
        assert!(!is_browser_session_connection(false, true, false));
    }

    #[test]
    fn authenticated_ws_is_always_browser_session() {
        assert!(is_browser_session_connection(true, false, false));
    }

    #[tokio::test]
    async fn unauthorized_ws_response_is_structured_json() {
        let response = unauthorized_ws_response(AuthErrorCode::TokenMissing);
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = body::to_bytes(response.into_body(), 1024).await.unwrap();
        let payload: AuthErrorResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload.code, AuthErrorCode::TokenMissing);
    }
}

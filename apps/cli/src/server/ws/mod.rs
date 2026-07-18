//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 08_auth#unauthorized-handling
//!
//! Authenticated WebSocket upgrade and session runtime entrypoint.

use axum::Json;
use axum::extract::{State, WebSocketUpgrade};
use axum::http::{HeaderValue, header::SET_COOKIE};
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
pub(crate) mod transport;

#[cfg(test)]
pub(crate) static WS_JSON_TEXT_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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
pub(crate) async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    axum::Extension(config): axum::Extension<Arc<AuthConfig>>,
    axum::Extension(admission_config): axum::Extension<Arc<WsAdmissionConfig>>,
    axum::Extension(transport_runtime): axum::Extension<Arc<transport::WsTransportRuntime>>,
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
    let Some(transport_permit) = transport_runtime.reserve_session() else {
        return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let browser_auth_session = admission.browser_auth_session().cloned();
    let set_cookie = admission.set_cookie().map(ToOwned::to_owned);
    let mut response = ws
        .on_upgrade(move |socket| {
            handle_socket(
                state,
                socket,
                peer_id,
                browser_auth_session,
                transport_permit,
            )
        })
        .into_response();
    if let Some(set_cookie) = set_cookie
        && let Ok(value) = HeaderValue::from_str(&set_cookie)
    {
        response.headers_mut().append(SET_COOKIE, value);
    }
    response
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
/// - **优先二进制帧**: 体积更小，解析更快，零字符串分配。
/// - **降级 JSON**: 向后兼容旧版客户端或调试场景。
pub(crate) async fn handle_socket(
    state: Arc<AppState>,
    socket: axum::extract::ws::WebSocket,
    peer_id: String,
    browser_auth_session: Option<crate::server::source_control_grants::AuthSessionId>,
    _transport_permit: transport::WsTransportSessionPermit,
) {
    let (sender, mut receiver) = socket.split();
    let mut session = WsSession::new();
    if let Some(auth_session_id) = browser_auth_session.clone() {
        session.mark_browser_session();
        session.bind_auth_session(auth_session_id);
    }

    // 为每个连接创建有界单播队列，避免慢客户端导致无界内存增长。
    let (unicast_tx, unicast_rx) = send::new_unicast_channel();
    let (diff_unicast_tx, diff_unicast_rx) = send::new_diff_unicast_channel();
    let (retire_session_tx, mut retire_session_rx) = tokio::sync::watch::channel(false);

    // 将单播队列写入 WebSocket。
    let unicast_task = send::spawn_unicast_sender_task(sender, unicast_rx, diff_unicast_rx);

    // 订阅广播并尝试转发到单播队列（带背压/丢弃策略）。
    let broadcast_rx = state.tx.subscribe();
    let broadcast_filter = send::BroadcastFilter::for_session(&session);
    let broadcast_task =
        send::spawn_broadcast_forwarder(broadcast_rx, unicast_tx.clone(), broadcast_filter.clone());

    let ch = DualChannel::with_diff_channel_and_retirement(
        state.tx.clone(),
        unicast_tx,
        diff_unicast_tx,
        retire_session_tx,
    );

    tracing::info!("Client connected: {}", peer_id);

    let mut transport_shutdown = _transport_permit.subscribe();
    loop {
        let next_message = receiver.next();
        tokio::pin!(next_message);
        let msg = tokio::select! {
            changed = retire_session_rx.changed() => {
                if changed.is_err() || *retire_session_rx.borrow() {
                    break;
                }
                continue;
            }
            changed = transport_shutdown.changed() => {
                if changed.is_err() || *transport_shutdown.borrow() {
                    break;
                }
                continue;
            }
            msg = &mut next_message => {
                let Some(msg) = msg else { break; };
                msg
            }
        };
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                tracing::warn!("WS receive error: {:?}", e);
                break;
            }
        };

        let handle_incoming = receive::handle_incoming_message(
            &state,
            &ch,
            &mut session,
            msg,
            &broadcast_filter,
            &peer_id,
        );
        tokio::pin!(handle_incoming);
        let flow = tokio::select! {
            changed = retire_session_rx.changed() => {
                if changed.is_err() || *retire_session_rx.borrow() {
                    break;
                }
                continue;
            }
            changed = transport_shutdown.changed() => {
                if changed.is_err() || *transport_shutdown.borrow() {
                    break;
                }
                continue;
            }
            flow = &mut handle_incoming => flow,
        };
        if matches!(flow, receive::SocketFlow::Break) {
            break;
        }
    }

    if let Some(auth_session_id) = browser_auth_session {
        state
            .source_control_write_grants()
            .revoke_session(&auth_session_id);
    }
    session.diff_projection_jobs.cancel();
    broadcast_task.abort();
    let _ = broadcast_task.await;
    unicast_task.abort();
    let _ = unicast_task.await;
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

use axum::extract::ws::Message;
use bincode::Options;
use std::sync::Arc;
use std::time::Instant;

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ClientMessage;
use deve_core::protocol::{ServerError, ServerErrorCode};

use super::route;
use super::send::BroadcastFilter;

/// Bincode 消息大小限制 (防止 DoS 攻击)。
const MAX_BINCODE_SIZE: u64 = 16 * 1024 * 1024;

pub(super) enum SocketFlow {
    Continue,
    Break,
}

pub(super) async fn handle_incoming_message(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: Message,
    broadcast_filter: &BroadcastFilter,
    peer_id: &str,
) -> SocketFlow {
    match msg {
        Message::Binary(bin) => {
            handle_binary(state, ch, session, &bin, broadcast_filter, peer_id).await
        }
        Message::Text(text) => {
            handle_text(state, ch, session, &text, broadcast_filter, peer_id).await
        }
        Message::Close(_) => {
            tracing::info!("Client disconnected: {}", peer_id);
            SocketFlow::Break
        }
        _ => SocketFlow::Continue,
    }
}

async fn handle_binary(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    bin: &[u8],
    broadcast_filter: &BroadcastFilter,
    peer_id: &str,
) -> SocketFlow {
    if !record_message(session, ch, peer_id) {
        return SocketFlow::Break;
    }
    let bincode_config = bincode::options().with_limit(MAX_BINCODE_SIZE);
    match bincode_config.deserialize::<ClientMessage>(bin) {
        Ok(client_msg) => {
            route::route_message(state, ch, session, client_msg).await;
            broadcast_filter.sync_from_session(session);
        }
        Err(e) => {
            tracing::warn!("Bincode parse error: {:?}, {} bytes", e, bin.len());
            ch.send_protocol_error_with_scope_nonce(
                invalid_client_message("Invalid bincode client message"),
                browser_scope_nonce(session),
            );
        }
    }
    SocketFlow::Continue
}

async fn handle_text(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    text: &str,
    broadcast_filter: &BroadcastFilter,
    peer_id: &str,
) -> SocketFlow {
    if !record_message(session, ch, peer_id) {
        return SocketFlow::Break;
    }
    match serde_json::from_str::<ClientMessage>(text) {
        Ok(client_msg) => {
            route::route_message(state, ch, session, client_msg).await;
            broadcast_filter.sync_from_session(session);
        }
        Err(_) => {
            tracing::warn!("Failed to parse client message: {}", text);
            ch.send_protocol_error_with_scope_nonce(
                invalid_client_message("Invalid JSON client message"),
                browser_scope_nonce(session),
            );
        }
    }
    SocketFlow::Continue
}

fn record_message(session: &mut WsSession, ch: &DualChannel, peer_id: &str) -> bool {
    if session.record_incoming_message(Instant::now()) {
        return true;
    }
    ch.send_protocol_error_with_scope_nonce(
        ServerError::with_detail(
            ServerErrorCode::RequestFailed,
            "WebSocket rate limit exceeded",
        ),
        browser_scope_nonce(session),
    );
    tracing::warn!("WS message rate limit exceeded: {}", peer_id);
    false
}

fn browser_scope_nonce(session: &WsSession) -> Option<u64> {
    session.is_browser_session().then(|| session.scope_nonce())
}

fn invalid_client_message(detail: &'static str) -> ServerError {
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

#[cfg(test)]
#[path = "receive_test.rs"]
mod tests;

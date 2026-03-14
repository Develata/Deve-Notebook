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
            ch.send_protocol_error(invalid_client_message("Invalid bincode client message"));
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
            ch.send_protocol_error(invalid_client_message("Invalid JSON client message"));
        }
    }
    SocketFlow::Continue
}

fn record_message(session: &mut WsSession, ch: &DualChannel, peer_id: &str) -> bool {
    if session.record_incoming_message(Instant::now()) {
        return true;
    }
    ch.send_protocol_error(ServerError::with_detail(
        ServerErrorCode::RequestFailed,
        "WebSocket rate limit exceeded",
    ));
    tracing::warn!("WS message rate limit exceeded: {}", peer_id);
    false
}

fn invalid_client_message(detail: &'static str) -> ServerError {
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

#[cfg(test)]
mod tests {
    use super::invalid_client_message;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn invalid_client_messages_use_structured_request_failed() {
        let error = invalid_client_message("Invalid JSON client message");
        assert_eq!(error.code, ServerErrorCode::RequestFailed);
        assert_eq!(error.detail.as_deref(), Some("Invalid JSON client message"));
    }
}

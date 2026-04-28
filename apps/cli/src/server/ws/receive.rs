//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! WebSocket inbound message decode and dispatch loop.

use axum::extract::ws::Message;
use std::sync::Arc;
use std::time::Instant;

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::frame::{
    MISSING_WS_FRAME_MAGIC, ProtocolFrameError, WsFrameFormat, decode_client_binary,
    decode_client_json_with_format,
};
use deve_core::protocol::{ServerError, ServerErrorCode};

use super::route;
use super::send::BroadcastFilter;

const LEGACY_JSON_TEXT_DISABLED_ERROR: &str =
    "Legacy JSON WS text frames are disabled outside development debug mode";

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
    match decode_client_binary(bin) {
        Ok(client_msg) => {
            route::route_message(state, ch, session, client_msg).await;
            broadcast_filter.sync_from_session(session);
        }
        Err(e) => {
            tracing::warn!("Bincode parse error: {:?}, {} bytes", e, bin.len());
            ch.send_protocol_error_with_scope_nonce(
                invalid_client_message(error_detail(&e, "Invalid bincode client message")),
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
    match decode_client_json_with_format(text) {
        Ok(decoded) => {
            if decoded.format == WsFrameFormat::LegacyJsonText {
                if !allow_legacy_json_text_debug() {
                    ch.send_protocol_error_with_scope_nonce(
                        invalid_client_message(LEGACY_JSON_TEXT_DISABLED_ERROR),
                        browser_scope_nonce(session),
                    );
                    return SocketFlow::Continue;
                }
                tracing::debug!("Accepted legacy JSON WS text debug frame");
            }
            route::route_message(state, ch, session, decoded.message).await;
            broadcast_filter.sync_from_session(session);
        }
        Err(e) => {
            tracing::warn!("Failed to parse client message: {}", text);
            ch.send_protocol_error_with_scope_nonce(
                invalid_client_message(error_detail(&e, "Invalid JSON client message")),
                browser_scope_nonce(session),
            );
        }
    }
    SocketFlow::Continue
}

fn allow_legacy_json_text_debug() -> bool {
    matches!(std::env::var("DEVE_ENV"), Ok(value) if value.eq_ignore_ascii_case("development"))
        || matches!(std::env::var("DEVE_ALLOW_LEGACY_WS_JSON"), Ok(value) if value == "1" || value.eq_ignore_ascii_case("true"))
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

fn invalid_client_message(detail: impl Into<String>) -> ServerError {
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

fn error_detail(error: &ProtocolFrameError, fallback: &'static str) -> String {
    match error {
        ProtocolFrameError::UnsupportedVersion { .. } => error.to_string(),
        ProtocolFrameError::Decode(detail) if detail == MISSING_WS_FRAME_MAGIC => detail.clone(),
        ProtocolFrameError::Decode(_) => fallback.to_string(),
    }
}

#[cfg(test)]
#[path = "receive_test.rs"]
mod tests;

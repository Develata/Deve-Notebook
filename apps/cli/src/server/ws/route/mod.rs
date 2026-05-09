//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Layered WebSocket message router.

use crate::server::handlers::sync;
use crate::server::metrics;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::ClientMessage;
use std::sync::Arc;

mod core;
mod core_scoped;
mod docs;
mod merge;
mod scope_guard;
mod source_control;

#[cfg(test)]
mod fallthrough_tests;

/// WebSocket 消息路由入口。
///
/// 通过分层路由把协议消息收敛到对应业务域，避免 route 层拥有 handler 语义。
pub(crate) async fn route_message(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) {
    metrics::increment_ops();
    match msg {
        ClientMessage::SyncHello {
            peer_id,
            peer_pubkey,
            session_proof,
            vector,
            repo_id,
            scope_nonce,
        } => {
            sync::handle_sync_hello(
                state,
                ch,
                session,
                sync::SyncHelloInput {
                    peer_id,
                    peer_pubkey,
                    session_proof,
                    remote_vector: vector,
                    repo_id,
                    scope_nonce: scope_nonce.get(),
                },
            )
            .await;
        }
        other => docs::route_docs(state, ch, session, other).await,
    }
}

use crate::server::handlers::sync;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::ClientMessage;
use std::sync::Arc;

mod core;
mod docs;
mod merge;
mod source_control;

/// WebSocket 消息路由入口。
///
/// 通过分层路由将大 match 拆分为多个小模块，
/// 以满足单文件行数限制并降低认知负担。
pub(crate) async fn route_message(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) {
    match msg {
        ClientMessage::SyncHello {
            peer_id,
            pub_key,
            signature,
            vector,
        } => {
            sync::handle_sync_hello(state, ch, peer_id, pub_key, signature, vector).await;
        }
        other => docs::route_docs(state, ch, session, other).await,
    }
}


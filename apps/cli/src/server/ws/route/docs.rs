//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Document structure WebSocket route.

use crate::server::handlers::docs;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::ClientMessage;
use std::sync::Arc;

/// 路由文档结构相关消息（创建/重命名/删除/复制/移动）。
pub(super) async fn route_docs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) {
    if let ClientMessage::DocumentCreate(request) = msg {
        docs::handle_create_doc_request(state, ch, session, request).await;
        return;
    }
    if let Some(scope) = msg.document_scope_gate()
        && super::scope_guard::reject_invalid_browser_scope_nonce(
            ch,
            session,
            scope.scope_nonce,
            scope.scope_name,
        )
    {
        return;
    }
    match msg {
        ClientMessage::RenameDoc {
            old_path, new_path, ..
        } => {
            docs::handle_rename_doc(state, ch, session, old_path, new_path).await;
        }
        ClientMessage::DeleteDoc { path, .. } => {
            docs::handle_delete_doc(state, ch, session, path).await;
        }
        ClientMessage::CopyDoc {
            src_path,
            dest_path,
            ..
        } => {
            docs::handle_copy_doc(state, ch, session, src_path, dest_path).await;
        }
        ClientMessage::MoveDoc {
            src_path,
            dest_path,
            ..
        } => {
            docs::handle_move_doc(state, ch, session, src_path, dest_path).await;
        }
        other => super::merge::route_merge(state, ch, session, other).await,
    }
}

#[cfg(test)]
mod tests;

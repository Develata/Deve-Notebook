//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Merge control WebSocket route.

use crate::server::handlers::merge;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::ClientMessage;
use std::sync::Arc;

/// 路由手动合并模式相关消息。
pub(super) async fn route_merge(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) {
    if let Some(scope) = msg.merge_control_scope_gate()
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
        ClientMessage::GetSyncMode { request_id, .. } => {
            merge::handle_get_sync_mode(state, ch, session, request_id).await;
        }
        ClientMessage::SetSyncMode { mode, .. } => {
            merge::handle_set_sync_mode(state, ch, session, mode).await;
        }
        ClientMessage::GetPendingOps { request_id, .. } => {
            merge::handle_get_pending_ops(state, ch, session, request_id).await;
        }
        ClientMessage::ConfirmMerge { .. } => {
            merge::handle_confirm_merge(state, ch, session).await;
        }
        ClientMessage::ResolveMergeConflict {
            doc_id,
            action,
            result_content,
            ..
        } => {
            merge::handle_resolve_merge_conflict(
                state,
                ch,
                session,
                doc_id,
                action,
                result_content,
            )
            .await;
        }
        ClientMessage::DiscardPending { .. } => {
            merge::handle_discard_pending(state, ch, session).await;
        }
        ClientMessage::MergePeer {
            peer_id, doc_id, ..
        } => {
            merge::handle_merge_peer(state, ch, session, peer_id, doc_id).await;
        }
        other => super::source_control::route_source_control(state, ch, session, other).await,
    }
}

#[cfg(test)]
#[path = "merge_readonly_test.rs"]
mod readonly_tests;

#[cfg(test)]
#[path = "merge_peer_test_support.rs"]
mod merge_peer_test_support;

#[cfg(test)]
#[path = "merge_peer_contract_test.rs"]
mod peer_contract_tests;

#[cfg(test)]
#[path = "merge_peer_resume_test.rs"]
mod peer_resume_tests;

#[cfg(test)]
#[path = "merge_test.rs"]
mod tests;

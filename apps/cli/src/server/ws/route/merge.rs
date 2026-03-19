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
    if let Some(scope_nonce) = requested_scope_nonce(&msg)
        && let Err(error) =
            super::scope_guard::validate_browser_scope_nonce(session, scope_nonce, "merge control")
    {
        ch.send_protocol_error_with_scope_nonce(
            error,
            super::scope_guard::response_scope_nonce(session, scope_nonce),
        );
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

fn requested_scope_nonce(msg: &ClientMessage) -> Option<Option<u64>> {
    match msg {
        ClientMessage::GetSyncMode { scope_nonce, .. }
        | ClientMessage::SetSyncMode { scope_nonce, .. }
        | ClientMessage::GetPendingOps { scope_nonce, .. }
        | ClientMessage::ConfirmMerge { scope_nonce }
        | ClientMessage::DiscardPending { scope_nonce }
        | ClientMessage::MergePeer { scope_nonce, .. } => Some(*scope_nonce),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::requested_scope_nonce;
    use deve_core::protocol::ClientMessage;

    #[test]
    fn extracts_scope_nonce_from_merge_messages() {
        assert_eq!(
            requested_scope_nonce(&ClientMessage::GetSyncMode {
                request_id: "req-1".into(),
                scope_nonce: Some(5),
            }),
            Some(Some(5))
        );
        assert_eq!(
            requested_scope_nonce(&ClientMessage::ConfirmMerge {
                scope_nonce: Some(7),
            }),
            Some(Some(7))
        );
        assert_eq!(requested_scope_nonce(&ClientMessage::Ping), None);
    }
}

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
    match msg {
        ClientMessage::GetSyncMode => {
            merge::handle_get_sync_mode(state, ch).await;
        }
        ClientMessage::SetSyncMode { mode } => {
            merge::handle_set_sync_mode(state, ch, mode).await;
        }
        ClientMessage::GetPendingOps => {
            merge::handle_get_pending_ops(state, ch).await;
        }
        ClientMessage::ConfirmMerge => {
            merge::handle_confirm_merge(state, ch).await;
        }
        ClientMessage::DiscardPending => {
            merge::handle_discard_pending(state, ch).await;
        }
        ClientMessage::MergePeer { peer_id, doc_id } => {
            merge::handle_merge_peer(state, ch, peer_id, doc_id).await;
        }
        other => super::source_control::route_source_control(state, ch, session, other).await,
    }
}

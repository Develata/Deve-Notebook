//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! 手动合并消息处理器入口。

mod errors;
mod manual;
mod manual_pending;
mod manual_support;
mod peer;
mod peer_apply;
mod peer_resolve;
mod peer_support;
mod scope;

use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::DocId;
use std::sync::Arc;

pub async fn handle_get_sync_mode(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
) {
    manual::handle_get_sync_mode(state, ch, session, request_id).await;
}

pub async fn handle_set_sync_mode(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    mode: String,
) {
    manual::handle_set_sync_mode(state, ch, session, mode).await;
}

pub async fn handle_get_pending_ops(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
) {
    manual_pending::handle_get_pending_ops(state, ch, session, request_id).await;
}

pub async fn handle_confirm_merge(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
) {
    manual_pending::handle_confirm_merge(state, ch, session).await;
}

pub async fn handle_discard_pending(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
) {
    manual_pending::handle_discard_pending(state, ch, session).await;
}

pub async fn handle_merge_peer(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: String,
    doc_id: DocId,
) {
    peer::handle_merge_peer(state, ch, session, peer_id, doc_id).await;
}

pub async fn handle_resolve_merge_conflict(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    doc_id: DocId,
    action: deve_core::protocol::MergeConflictAction,
    result_content: Option<String>,
) {
    peer_resolve::handle_resolve_merge_conflict(state, ch, session, doc_id, action, result_content)
        .await;
}

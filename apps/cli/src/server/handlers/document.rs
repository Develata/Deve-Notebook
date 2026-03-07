//! 文档消息处理器入口。

mod edit;
mod history;
mod open;
mod snapshot;

use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::{DocId, Op};
use std::sync::Arc;

pub async fn handle_edit(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    doc_id: DocId,
    op: Op,
    client_id: u64,
) {
    edit::handle_edit(state, ch, session, doc_id, op, client_id).await;
}

#[allow(dead_code)]
pub async fn handle_request_history(state: &Arc<AppState>, ch: &DualChannel, doc_id: DocId) {
    history::handle_request_history(state, ch, doc_id).await;
}

pub async fn handle_open_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    doc_id: DocId,
) {
    open::handle_open_doc(state, ch, session, doc_id).await;
}

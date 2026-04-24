//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime

use super::{
    AppState, channel::DualChannel, handlers::document::handle_edit, session::WsSession,
};
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ServerError, ServerMessage};
use std::sync::Arc;
use tokio::sync::mpsc;

pub(crate) async fn send_insert(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    doc_id: DocId,
    pos: u32,
) {
    handle_edit(
        state,
        ch,
        session,
        doc_id,
        Op::Insert {
            pos,
            content: "!".into(),
        },
        7,
        9,
    )
    .await;
}

pub(crate) async fn recv_edit_rejected(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> (Option<u64>, DocId, u64, ServerError) {
    match rx.recv().await {
        Some(ServerMessage::EditRejected {
            scope_nonce,
            doc_id,
            client_op_id,
            error,
        }) => (scope_nonce, doc_id, client_op_id, error),
        other => panic!("expected EditRejected, got {:?}", other),
    }
}

pub(crate) async fn recv_ack(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> (Option<u64>, DocId, u64) {
    match rx.recv().await {
        Some(ServerMessage::Ack {
            scope_nonce,
            doc_id,
            client_op_id,
            ..
        }) => (scope_nonce, doc_id, client_op_id),
        other => panic!("expected Ack, got {:?}", other),
    }
}

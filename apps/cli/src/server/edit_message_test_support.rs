//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime

use super::{
    AppState,
    channel::DualChannel,
    handlers::document::{EditRequest, handle_edit},
    session::WsSession,
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
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    send_insert_with_scope(state, ch, session, doc_id, pos, scope_nonce).await;
}

pub(crate) async fn send_insert_with_scope(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    doc_id: DocId,
    pos: u32,
    scope_nonce: Option<u64>,
) {
    handle_edit(
        state,
        ch,
        session,
        EditRequest {
            doc_id,
            op: Op::Insert {
                pos,
                content: "!".into(),
            },
            client_id: 7,
            client_op_id: 9,
            scope_nonce,
        },
    )
    .await;
}

pub(crate) async fn recv_edit_rejected(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> (u64, DocId, u64, ServerError) {
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

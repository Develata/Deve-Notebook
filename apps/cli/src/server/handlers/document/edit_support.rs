//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!
//! Shared edit-scope resolution and document edit response helpers.

use crate::server::repo_scope::{
    ResolvedRepo, map_repo_scope_error, resolve_session_repo_or_bootstrap_local,
};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ClientOrigin, ConfirmedOp, ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

pub(super) struct CommittedEdit {
    pub(super) scope_nonce: u64,
    pub(super) doc_id: DocId,
    pub(super) global_seq: u64,
    pub(super) op: Op,
    pub(super) client_id: u64,
    pub(super) client_op_id: u64,
}

pub(super) fn edit_response_scope_nonce(
    session: &WsSession,
    requested_scope_nonce: Option<u64>,
) -> u64 {
    requested_scope_nonce.unwrap_or_else(|| session.scope_nonce())
}

pub(super) fn resolve_edit_scope(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope_nonce: u64,
    doc_id: DocId,
    client_op_id: u64,
) -> Option<ResolvedRepo> {
    match resolve_session_repo_or_bootstrap_local(state, session) {
        Ok(scope) => {
            if scope.branch.is_none()
                && (session.active_repo.as_deref() != Some(scope.repo_name.as_str())
                    || session.active_repo_id != Some(scope.repo_id))
            {
                session.switch_repo(scope.repo_name.clone(), Some(scope.repo_id));
            }
            Some(scope)
        }
        Err(err) => {
            reject_edit(
                ch,
                scope_nonce,
                doc_id,
                client_op_id,
                map_repo_scope_error(err),
            );
            None
        }
    }
}

pub(super) fn reject_edit(
    ch: &DualChannel,
    scope_nonce: u64,
    doc_id: DocId,
    client_op_id: u64,
    error: ServerError,
) {
    ch.unicast(ServerMessage::EditRejected {
        scope_nonce: scope_nonce.into(),
        doc_id,
        client_op_id,
        error,
    });
}

pub(super) fn broadcast_and_ack_committed_edit(
    ch: &DualChannel,
    scope: &ResolvedRepo,
    edit: CommittedEdit,
) {
    ch.broadcast(ServerMessage::NewOp {
        repo_id: scope.repo_id,
        branch: scope.branch.clone(),
        scope_nonce: Some(edit.scope_nonce),
        doc_id: edit.doc_id,
        entry: ConfirmedOp::new(
            edit.global_seq,
            edit.op,
            Some(ClientOrigin {
                client_id: edit.client_id,
                client_op_id: edit.client_op_id,
            }),
        ),
    });
    ch.unicast(ServerMessage::Ack {
        repo_id: scope.repo_id,
        branch: scope.branch.clone(),
        scope_nonce: Some(edit.scope_nonce),
        doc_id: edit.doc_id,
        seq: edit.global_seq,
        client_op_id: edit.client_op_id,
    });
}

pub(super) fn report_projection_writeback_fault(
    ch: &DualChannel,
    scope_nonce: Option<u64>,
    detail: impl Into<String>,
) {
    ch.unicast(ServerMessage::ProtocolError {
        error: ServerError::with_detail(ServerErrorCode::StoragePersistFailed, detail),
        switch_nonce: None,
        scope_nonce,
    });
}

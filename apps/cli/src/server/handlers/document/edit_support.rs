use crate::server::repo_scope::{
    ResolvedRepo, map_repo_scope_error, resolve_session_repo_or_bootstrap_local,
};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ClientOrigin, ConfirmedOp, ServerError, ServerMessage};
use std::sync::Arc;

pub(super) fn resolve_edit_scope(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope_nonce: Option<u64>,
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
    scope_nonce: Option<u64>,
    doc_id: DocId,
    client_op_id: u64,
    error: ServerError,
) {
    ch.unicast(ServerMessage::EditRejected {
        scope_nonce,
        doc_id,
        client_op_id,
        error,
    });
}

pub(super) fn broadcast_and_ack_committed_edit(
    ch: &DualChannel,
    scope: &ResolvedRepo,
    scope_nonce: Option<u64>,
    doc_id: DocId,
    local_seq: u64,
    op: Op,
    client_id: u64,
    client_op_id: u64,
) {
    ch.broadcast(ServerMessage::NewOp {
        repo_id: scope.repo_id,
        branch: scope.branch.clone(),
        scope_nonce,
        doc_id,
        entry: ConfirmedOp::new(
            local_seq,
            op,
            Some(ClientOrigin {
                client_id,
                client_op_id,
            }),
        ),
    });
    ch.unicast(ServerMessage::Ack {
        repo_id: scope.repo_id,
        branch: scope.branch.clone(),
        scope_nonce,
        doc_id,
        seq: local_seq,
        client_op_id,
    });
}

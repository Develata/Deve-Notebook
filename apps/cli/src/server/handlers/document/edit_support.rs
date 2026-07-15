//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!
//! Shared edit-scope resolution and document edit response helpers.

use crate::server::repo_scope::{
    ResolvedRepo, map_repo_scope_error, resolve_session_repo_or_bootstrap_local,
};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::DocId;
use deve_core::protocol::{ServerError, ServerMessage};
use std::sync::Arc;

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

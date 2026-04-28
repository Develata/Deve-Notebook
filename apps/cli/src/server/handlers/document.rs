//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
//! 文档消息处理器入口。

mod confirmed;
mod edit;
mod edit_apply;
mod edit_checks;
mod edit_support;
pub(crate) mod errors;
mod history;
mod open;
mod snapshot;
mod snapshot_delta_guard;
#[cfg(test)]
mod snapshot_delta_guard_test;

use crate::server::repo_scope::{
    ResolvedRepo, map_repo_scope_error, resolve_session_repo_or_bootstrap_local,
};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::{DocId, Op};
use std::sync::Arc;

pub async fn handle_edit(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    doc_id: DocId,
    op: Op,
    client_id: u64,
    client_op_id: u64,
) {
    edit::handle_edit(state, ch, session, doc_id, op, client_id, client_op_id).await;
}

pub async fn handle_request_history(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    doc_id: DocId,
    request_id: u64,
) {
    history::handle_request_history(state, ch, session, doc_id, request_id).await;
}

pub async fn handle_open_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    doc_id: DocId,
    request_id: u64,
) {
    open::handle_open_doc(state, ch, session, doc_id, request_id).await;
}

pub(super) fn resolve_document_scope(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope_nonce: Option<u64>,
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
            ch.send_protocol_error_with_scope_nonce(map_repo_scope_error(err), scope_nonce);
            None
        }
    }
}

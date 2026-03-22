//! 文档消息处理器入口。

mod confirmed;
mod edit;
pub(crate) mod errors;
mod history;
mod open;
mod snapshot;

use crate::server::repo_scope::{
    ResolvedRepo, bootstrap_local_repo, map_repo_scope_error, resolve_session_repo_and_sync,
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

#[allow(dead_code)]
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
    if session.active_branch.is_none()
        && session.active_repo.is_none()
        && session.active_repo_id.is_none()
        && !session.has_runtime_scope_binding()
    {
        let scope = match bootstrap_local_repo(state, session) {
            Ok(scope) => scope,
            Err(err) => {
                ch.send_protocol_error_with_scope_nonce(map_repo_scope_error(err), scope_nonce);
                return None;
            }
        };
        session.switch_repo(scope.repo_name.clone(), Some(scope.repo_id));
        return Some(scope);
    }
    match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => Some(scope),
        Err(err) => {
            ch.send_protocol_error_with_scope_nonce(map_repo_scope_error(err), scope_nonce);
            None
        }
    }
}

//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!
//! Applies browser edit requests through the document authority bridge.

use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::{DocId, Op};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

use super::edit_apply::append_client_edit;
use super::edit_checks::{confirm_existing_client_op, reject_missing_doc, writer_peer_id};
use super::edit_support::{reject_edit, resolve_edit_scope};

pub(super) async fn handle_edit(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    doc_id: DocId,
    op: Op,
    client_id: u64,
    client_op_id: u64,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let Some(scope) = resolve_edit_scope(state, ch, session, scope_nonce, doc_id, client_op_id)
    else {
        return;
    };
    if scope.branch.is_some() {
        tracing::debug!("Edit rejected: resolved scope is readonly (remote branch)");
        reject_edit(
            ch,
            scope_nonce,
            doc_id,
            client_op_id,
            ServerError::new(ServerErrorCode::ScRemoteBranchReadonly),
        );
        return;
    }
    if let Err(error) = reject_missing_doc(state, &scope.repo_name, doc_id) {
        reject_edit(ch, scope_nonce, doc_id, client_op_id, error);
        return;
    }
    let Some(local_peer_id) = writer_peer_id(
        session,
        &scope.repo_id,
        doc_id,
        client_op_id,
        ch,
        scope_nonce,
    ) else {
        return;
    };
    if confirm_existing_client_op(
        state,
        &scope,
        ch,
        scope_nonce,
        doc_id,
        &op,
        client_id,
        client_op_id,
    ) {
        return;
    }
    append_client_edit(
        state,
        &scope,
        ch,
        scope_nonce,
        doc_id,
        op,
        local_peer_id,
        client_id,
        client_op_id,
    );
}

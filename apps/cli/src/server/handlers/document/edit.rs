//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!
//! Applies browser edit requests through the document authority bridge.

use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

use super::EditRequest;
use super::edit_apply::{ClientEditAppend, append_client_edit};
use super::edit_checks::{
    ExistingClientOpCheck, confirm_existing_client_op, reject_missing_doc, writer_peer_id,
};
use super::edit_support::{reject_edit, resolve_edit_scope};

pub(super) async fn handle_edit(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request: EditRequest,
) {
    let EditRequest {
        doc_id,
        op,
        client_id,
        client_op_id,
        scope_nonce,
    } = request;
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
    if confirm_existing_client_op(ExistingClientOpCheck {
        state,
        scope: &scope,
        ch,
        scope_nonce,
        doc_id,
        op: &op,
        client_id,
        client_op_id,
    }) {
        return;
    }
    append_client_edit(ClientEditAppend {
        state,
        scope: &scope,
        ch,
        scope_nonce,
        doc_id,
        op,
        local_peer_id,
        client_id,
        client_op_id,
    });
}

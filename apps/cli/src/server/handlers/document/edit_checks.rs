//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!
//! Fail-closed validation before document edits reach ledger authority.

use crate::server::{AppState, channel::DualChannel, repo_scope::ResolvedRepo, session::WsSession};
use deve_core::models::{DocId, Op, PeerId};
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

use super::edit_support::reject_edit;

pub(super) fn reject_missing_doc(
    state: &Arc<AppState>,
    repo_name: &str,
    doc_id: DocId,
) -> Result<(), ServerError> {
    match state
        .repo
        .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)
    {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(ServerError::with_detail(
            ServerErrorCode::DocNotFound,
            format!("Document not found in active repository: {doc_id}"),
        )),
        Err(err) => Err(ServerError::with_detail(
            ServerErrorCode::StoragePersistFailed,
            err.to_string(),
        )),
    }
}

pub(super) fn writer_peer_id(
    session: &WsSession,
    repo_id: &uuid::Uuid,
    doc_id: DocId,
    client_op_id: u64,
    ch: &DualChannel,
    requested_scope_nonce: Option<u64>,
    response_scope_nonce: u64,
) -> Option<PeerId> {
    session
        .writer_peer_id_for(repo_id, requested_scope_nonce)
        .or_else(|| {
            reject_edit(
                ch,
                response_scope_nonce,
                doc_id,
                client_op_id,
                ServerError::new(ServerErrorCode::SyncPeerUnauthenticated),
            );
            None
        })
}

pub(super) struct ExistingClientOpCheck<'a> {
    pub(super) state: &'a Arc<AppState>,
    pub(super) scope: &'a ResolvedRepo,
    pub(super) ch: &'a DualChannel,
    pub(super) scope_nonce: u64,
    pub(super) doc_id: DocId,
    pub(super) op: &'a Op,
    pub(super) client_id: u64,
    pub(super) client_op_id: u64,
}

pub(super) fn confirm_existing_client_op(input: ExistingClientOpCheck<'_>) -> bool {
    let ExistingClientOpCheck {
        state,
        scope,
        ch,
        scope_nonce,
        doc_id,
        op,
        client_id,
        client_op_id,
    } = input;
    match state
        .repo
        .find_client_op_in_local_repo(&scope.repo_name, client_id, client_op_id)
    {
        Ok(Some((_global_seq, entry))) if entry.content_op() == Some(op) => {
            let Some(ack_doc_id) = entry.doc_id else {
                reject_edit(
                    ch,
                    scope_nonce,
                    doc_id,
                    client_op_id,
                    ServerError::with_detail(
                        ServerErrorCode::StoragePersistFailed,
                        "Broken client op index: indexed entry missing doc id",
                    ),
                );
                return true;
            };
            ch.unicast(ServerMessage::Ack {
                repo_id: scope.repo_id,
                branch: scope.branch.clone(),
                scope_nonce: Some(scope_nonce),
                doc_id: ack_doc_id,
                seq: entry.seq,
                client_op_id,
            });
            true
        }
        Ok(Some(_)) => {
            reject_edit(
                ch,
                scope_nonce,
                doc_id,
                client_op_id,
                ServerError::with_detail(
                    ServerErrorCode::SyncEditRejected,
                    "client_op_id conflicts with a different op",
                ),
            );
            true
        }
        Ok(None) => false,
        Err(err) => {
            reject_edit(
                ch,
                scope_nonce,
                doc_id,
                client_op_id,
                ServerError::with_detail(ServerErrorCode::StoragePersistFailed, err.to_string()),
            );
            true
        }
    }
}

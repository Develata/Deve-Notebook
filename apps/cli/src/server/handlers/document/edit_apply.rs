//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!
//! Persists accepted document edits and emits ack/broadcast messages.

use crate::server::{AppState, channel::DualChannel, repo_scope::ResolvedRepo};
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

use super::edit_support::{CommittedEdit, broadcast_and_ack_committed_edit, reject_edit};

pub(super) struct ClientEditAppend<'a> {
    pub(super) state: &'a Arc<AppState>,
    pub(super) scope: &'a ResolvedRepo,
    pub(super) ch: &'a DualChannel,
    pub(super) scope_nonce: Option<u64>,
    pub(super) doc_id: DocId,
    pub(super) op: Op,
    pub(super) local_peer_id: PeerId,
    pub(super) client_id: u64,
    pub(super) client_op_id: u64,
}

pub(super) fn append_client_edit(input: ClientEditAppend<'_>) {
    let ClientEditAppend {
        state,
        scope,
        ch,
        scope_nonce,
        doc_id,
        op,
        local_peer_id,
        client_id,
        client_op_id,
    } = input;
    let op_clone = op.clone();
    let peer_id_clone = local_peer_id.clone();
    match state.repo.append_generated_client_op_in_local_repo(
        &scope.repo_name,
        doc_id,
        local_peer_id,
        client_id,
        client_op_id,
        move |seq| {
            LedgerEntry::new_content(
                doc_id,
                op_clone.clone(),
                chrono::Utc::now().timestamp_millis(),
                peer_id_clone.clone(),
                seq,
                Some(client_id),
                Some(client_op_id),
            )
        },
    ) {
        Ok((_global_seq, local_seq)) => {
            if let Err(err) = state
                .sync_manager
                .persist_doc_in_local_repo(&scope.repo_name, doc_id)
            {
                tracing::error!(
                    doc_id = %doc_id,
                    client_op_id,
                    "Workspace projection writeback failed after ledger commit: {:?}",
                    err
                );
            }
            broadcast_and_ack_committed_edit(
                ch,
                scope,
                CommittedEdit {
                    scope_nonce,
                    doc_id,
                    local_seq,
                    op,
                    client_id,
                    client_op_id,
                },
            );
        }
        Err(err) => {
            tracing::error!("Failed to persist op: {:?}", err);
            reject_edit(
                ch,
                scope_nonce,
                doc_id,
                client_op_id,
                ServerError::with_detail(ServerErrorCode::StoragePersistFailed, err.to_string()),
            );
        }
    }
}

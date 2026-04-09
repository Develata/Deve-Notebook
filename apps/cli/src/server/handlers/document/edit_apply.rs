use crate::server::{AppState, channel::DualChannel, repo_scope::ResolvedRepo};
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

use super::edit_support::{broadcast_and_ack_committed_edit, reject_edit};

#[allow(clippy::too_many_arguments)]
pub(super) fn append_client_edit(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    ch: &DualChannel,
    scope_nonce: Option<u64>,
    doc_id: DocId,
    op: Op,
    local_peer_id: PeerId,
    client_id: u64,
    client_op_id: u64,
) {
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
                scope_nonce,
                doc_id,
                local_seq,
                op,
                client_id,
                client_op_id,
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

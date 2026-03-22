use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::{DocId, LedgerEntry, Op};
use deve_core::protocol::{ClientOrigin, ConfirmedOp, ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

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
    let Some(scope) = super::resolve_document_scope(state, ch, session, scope_nonce) else {
        return;
    };
    if scope.branch.is_some() {
        tracing::debug!("Edit rejected: resolved scope is readonly (remote branch)");
        ch.unicast(ServerMessage::EditRejected {
            scope_nonce,
            error: ServerError::new(ServerErrorCode::ScRemoteBranchReadonly),
        });
        return;
    }
    match state
        .repo
        .get_file_meta_for_doc_in_local_repo(&scope.repo_name, doc_id)
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            ch.unicast(ServerMessage::EditRejected {
                scope_nonce,
                error: ServerError::with_detail(
                    ServerErrorCode::StorageNotFound,
                    format!("Document not found in active repository: {doc_id}"),
                ),
            });
            return;
        }
        Err(e) => {
            ch.send_protocol_error_with_scope_nonce(
                ServerError::with_detail(ServerErrorCode::StoragePersistFailed, e.to_string()),
                scope_nonce,
            );
            return;
        }
    }
    let local_peer_id = match session.writer_peer_id_for(&scope.repo_id) {
        Some(peer_id) => peer_id,
        None => {
            ch.send_protocol_error_with_scope_nonce(
                ServerError::new(ServerErrorCode::SyncPeerUnauthenticated),
                scope_nonce,
            );
            return;
        }
    };
    match state
        .repo
        .find_client_op_in_local_repo(&scope.repo_name, doc_id, client_id, client_op_id)
    {
        Ok(Some((_global_seq, entry))) => {
            if entry.content_op() != Some(&op) {
                ch.send_protocol_error_with_scope_nonce(
                    ServerError::with_detail(
                        ServerErrorCode::SyncEditRejected,
                        "client_op_id conflicts with a different op",
                    ),
                    scope_nonce,
                );
                return;
            }
            ch.unicast(ServerMessage::Ack {
                repo_id: scope.repo_id,
                branch: scope.branch.clone(),
                scope_nonce,
                doc_id,
                seq: entry.seq,
                client_op_id,
            });
            return;
        }
        Ok(None) => {}
        Err(err) => {
            ch.send_protocol_error_with_scope_nonce(
                ServerError::with_detail(ServerErrorCode::StoragePersistFailed, err.to_string()),
                scope_nonce,
            );
            return;
        }
    }
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
            if let Err(e) = state
                .sync_manager
                .persist_doc_in_local_repo(&scope.repo_name, doc_id)
            {
                tracing::error!("Failed to persist op: {:?}", e);
                ch.send_protocol_error_with_scope_nonce(
                    ServerError::with_detail(ServerErrorCode::StoragePersistFailed, e.to_string()),
                    scope_nonce,
                );
                return;
            }
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
        Err(e) => {
            tracing::error!("Failed to persist op: {:?}", e);
            ch.send_protocol_error_with_scope_nonce(
                ServerError::with_detail(ServerErrorCode::StoragePersistFailed, e.to_string()),
                scope_nonce,
            );
        }
    }
}

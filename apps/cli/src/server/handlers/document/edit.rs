use crate::server::repo_scope::resolve_session_repo_and_sync;
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
    if session.is_readonly() {
        tracing::debug!("Edit rejected: session is readonly (remote branch)");
        ch.unicast(ServerMessage::EditRejected {
            error: ServerError::new(ServerErrorCode::ScRemoteBranchReadonly),
        });
        return;
    }
    let scope = match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => scope,
        Err(e) => {
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::SyncEditRejected,
                e.to_string(),
            ));
            return;
        }
    };
    let local_peer_id = match session.writer_peer_id_for(&scope.repo_id) {
        Some(peer_id) => peer_id,
        None => {
            ch.send_protocol_error(ServerError::new(ServerErrorCode::SyncPeerUnauthenticated));
            return;
        }
    };
    if let Ok(Some((_global_seq, entry))) =
        state
            .repo
            .find_client_op_in_local_repo(&scope.repo_name, doc_id, client_id, client_op_id)
    {
        if entry.content_op() != Some(&op) {
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::SyncEditRejected,
                "client_op_id conflicts with a different op",
            ));
            return;
        }
        ch.unicast(ServerMessage::Ack {
            doc_id,
            seq: entry.seq,
            client_op_id,
        });
        return;
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
                ch.send_protocol_error(ServerError::with_detail(
                    ServerErrorCode::StoragePersistFailed,
                    e.to_string(),
                ));
                return;
            }
            ch.broadcast(ServerMessage::NewOp {
                repo_id: scope.repo_id,
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
                doc_id,
                seq: local_seq,
                client_op_id,
            });
        }
        Err(e) => {
            tracing::error!("Failed to persist op: {:?}", e);
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::StoragePersistFailed,
                e.to_string(),
            ));
        }
    }
}

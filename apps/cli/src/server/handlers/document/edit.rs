use crate::server::repo_scope::resolve_session_repo;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::{DocId, LedgerEntry, Op};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub(super) async fn handle_edit(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    doc_id: DocId,
    op: Op,
    client_id: u64,
    client_op_id: u64,
) {
    if session.is_readonly() {
        tracing::debug!("Edit ignored: session is readonly (remote branch)");
        return;
    }
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    let local_peer_id = match session.authenticated_peer_id.clone() {
        Some(peer_id) => peer_id,
        None => {
            ch.send_error("Browser peer not authenticated for edit".to_string());
            return;
        }
    };
    let op_clone = op.clone();
    let peer_id_clone = local_peer_id.clone();
    match state.sync_manager.apply_local_op_in_local_repo(
        &scope.repo_name,
        doc_id,
        local_peer_id,
        move |seq| LedgerEntry {
            doc_id,
            op: op_clone.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            peer_id: peer_id_clone.clone(),
            seq,
        },
        true,
    ) {
        Ok((_global_seq, local_seq)) => {
            ch.broadcast(ServerMessage::NewOp {
                doc_id,
                op,
                seq: local_seq,
                client_id,
            });
            ch.unicast(ServerMessage::Ack {
                doc_id,
                seq: local_seq,
                client_op_id,
            });
        }
        Err(e) => {
            tracing::error!("Failed to persist op: {:?}", e);
            ch.send_error(format!("Failed to persist operation: {}", e));
        }
    }
}

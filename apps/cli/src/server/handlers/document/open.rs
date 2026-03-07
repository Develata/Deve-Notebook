use super::snapshot::{SnapshotPayload, build_snapshot_payload};
use crate::server::repo_scope::resolve_session_repo;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::DocId;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;
use std::time::Instant;

pub(super) async fn handle_open_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    doc_id: DocId,
) {
    tracing::info!(
        "OpenDoc Request for DocID: {}, Branch: {:?}, Repo: {:?}",
        doc_id,
        session.active_branch,
        session.active_repo
    );

    let start = Instant::now();
    let (content, base_seq, delta_ops, version) = if let Some(handle) = session.get_active_db() {
        match build_snapshot_payload(&handle.db, doc_id, state.repo.snapshot_depth) {
            Ok(payload) => payload,
            Err(e) => {
                tracing::error!("Failed to build snapshot from active_db: {:?}", e);
                empty_payload()
            }
        }
    } else {
        load_snapshot_from_repo(state, session, doc_id)
    };

    tracing::info!(
        "OpenDoc Prepared: doc={}, base_seq={}, version={}, pending_ops={}, elapsed_ms={}",
        doc_id,
        base_seq,
        version,
        delta_ops.len(),
        start.elapsed().as_millis()
    );

    ch.unicast(ServerMessage::Snapshot {
        doc_id,
        content,
        base_seq,
        version,
        delta_ops,
    });
}

fn load_snapshot_from_repo(
    state: &Arc<AppState>,
    session: &WsSession,
    doc_id: DocId,
) -> SnapshotPayload {
    let repo_name = resolve_session_repo(state, session)
        .map(|scope| scope.repo_name)
        .unwrap_or_else(|_| state.repo.local_repo_name().to_string());
    tracing::warn!(
        "No active_db in session, falling back to resolved local repo {}",
        repo_name
    );
    if let Err(e) = state
        .sync_manager
        .reconcile_doc_in_local_repo(&repo_name, doc_id)
    {
        tracing::error!("SyncManager reconcile failed: {:?}", e);
    }

    match state.repo.run_on_local_repo(&repo_name, |db| {
        build_snapshot_payload(db, doc_id, state.repo.snapshot_depth)
    }) {
        Ok(payload) => payload,
        Err(e) => {
            tracing::error!("Failed to read snapshot from repo {}: {:?}", repo_name, e);
            empty_payload()
        }
    }
}

fn empty_payload() -> SnapshotPayload {
    (String::new(), 0, Vec::new(), 0)
}

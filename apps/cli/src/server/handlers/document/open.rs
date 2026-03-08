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
    let (content, base_seq, delta_ops, version) = match session.get_active_db() {
        Some(handle) => match build_snapshot_payload(&handle.db, doc_id, state.repo.snapshot_depth)
        {
            Ok(payload) => payload,
            Err(e) => {
                tracing::error!("Failed to build snapshot from active_db: {:?}", e);
                empty_payload()
            }
        },
        None if session.active_branch.is_none() => {
            match load_snapshot_from_local_repo(state, session, doc_id) {
                Ok(payload) => payload,
                Err(e) => {
                    ch.send_error(e.to_string());
                    empty_payload()
                }
            }
        }
        None => {
            ch.send_error("Remote repository database is not locked".to_string());
            empty_payload()
        }
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

fn load_snapshot_from_local_repo(
    state: &Arc<AppState>,
    session: &WsSession,
    doc_id: DocId,
) -> anyhow::Result<SnapshotPayload> {
    let scope = resolve_session_repo(state, session)?;
    state.repo.run_on_local_repo(&scope.repo_name, |db| {
        build_snapshot_payload(db, doc_id, state.repo.snapshot_depth)
    })
}

fn empty_payload() -> SnapshotPayload {
    (String::new(), 0, Vec::new(), 0)
}

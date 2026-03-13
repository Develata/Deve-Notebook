use super::errors::send_doc_error;
use super::snapshot::{SnapshotPayload, build_snapshot_payload};
use crate::server::repo_scope::{map_repo_scope_error, resolve_session_repo_and_sync};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;
use std::time::Instant;

pub(super) async fn handle_open_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    doc_id: DocId,
    request_id: u64,
) {
    tracing::info!(
        "OpenDoc Request for DocID: {}, Branch: {:?}, Repo: {:?}",
        doc_id,
        session.active_branch,
        session.active_repo
    );

    let start = Instant::now();
    let scope = match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => scope,
        Err(e) => {
            ch.send_protocol_error(map_repo_scope_error(e));
            return;
        }
    };
    let (content, base_seq, delta_ops, version) =
        match load_snapshot(state, session, &scope.repo_name, scope.repo_id, doc_id) {
            Ok(payload) => payload,
            Err(e) => {
                send_doc_error(ch, "Failed to load document snapshot", e);
                return;
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
        repo_id: scope.repo_id,
        branch: session.active_branch.clone(),
        doc_id,
        request_id,
        content,
        base_seq,
        version,
        delta_ops,
    });
}

fn load_snapshot_from_local_repo(
    state: &Arc<AppState>,
    repo_name: &str,
    doc_id: DocId,
) -> anyhow::Result<SnapshotPayload> {
    state.repo.run_on_local_repo(repo_name, |db| {
        build_snapshot_payload(db, doc_id, state.repo.snapshot_depth)
    })
}

fn load_snapshot(
    state: &Arc<AppState>,
    session: &WsSession,
    repo_name: &str,
    repo_id: RepoId,
    doc_id: DocId,
) -> anyhow::Result<SnapshotPayload> {
    if let Some(peer_id) = session.active_branch.as_ref() {
        return load_snapshot_from_shadow_repo(state, peer_id, repo_id, doc_id);
    }
    if let Some(handle) = session.active_db_for(None, repo_name, Some(repo_id)) {
        return build_snapshot_payload(&handle.db, doc_id, state.repo.snapshot_depth);
    }
    load_snapshot_from_local_repo(state, repo_name, doc_id)
}

fn load_snapshot_from_shadow_repo(
    state: &Arc<AppState>,
    peer_id: &PeerId,
    repo_id: RepoId,
    doc_id: DocId,
) -> anyhow::Result<SnapshotPayload> {
    state
        .repo
        .run_on_shadow_repo_by_id(peer_id, &repo_id, |db| {
            build_snapshot_payload(db, doc_id, state.repo.snapshot_depth)
        })
}

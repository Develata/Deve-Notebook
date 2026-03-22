use super::errors::send_doc_error_with_scope_nonce;
use super::snapshot::{SnapshotPayload, build_snapshot_payload};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::{DocId, PeerId, RepoId, RepoType};
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
    let scope_nonce = browser_scope_nonce(session);
    let Some(scope) = super::resolve_document_scope(state, ch, session, scope_nonce) else {
        return;
    };
    let (content, base_seq, delta_ops, version) = match load_snapshot(state, &scope, doc_id) {
        Ok(payload) => payload,
        Err(e) => {
            send_doc_error_with_scope_nonce(ch, "Failed to load document snapshot", e, scope_nonce);
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
        branch: scope.branch.clone(),
        scope_nonce,
        doc_id,
        request_id,
        content,
        base_seq,
        version,
        delta_ops,
    });
}

fn browser_scope_nonce(session: &WsSession) -> Option<u64> {
    session.is_browser_session().then(|| session.scope_nonce())
}

fn load_snapshot(
    state: &Arc<AppState>,
    scope: &super::ResolvedRepo,
    doc_id: DocId,
) -> anyhow::Result<SnapshotPayload> {
    state.repo.run_on_repo_db(
        &resolved_repo_type(scope.branch.as_ref(), scope.repo_id),
        |db| build_snapshot_payload(db, doc_id, state.repo.snapshot_depth),
    )
}

fn resolved_repo_type(branch: Option<&PeerId>, repo_id: RepoId) -> RepoType {
    match branch.cloned() {
        Some(peer_id) => RepoType::Remote(peer_id, repo_id),
        None => RepoType::Local(repo_id),
    }
}

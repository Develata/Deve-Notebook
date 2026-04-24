//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!
//! Snapshot-first OpenDoc handler.

use super::errors::{OpenDocErrorContext, send_open_doc_error_with_scope_nonce};
use super::snapshot::{SnapshotPayload, build_snapshot_payload};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::ledger::ops::{local_repo_scope, shadow_repo_scope};
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
        doc_id = %doc_id,
        request_id,
        active_branch = ?session.active_branch,
        active_repo = ?session.active_repo,
        active_repo_id = ?session.active_repo_id,
        "OpenDoc request received"
    );

    let start = Instant::now();
    let scope_nonce = browser_scope_nonce(session);
    let Some(scope) = super::resolve_document_scope(state, ch, session, scope_nonce) else {
        return;
    };
    let repo_scope = resolved_repo_scope(&scope);
    tracing::info!(
        repo_scope = repo_scope.as_str(),
        doc_id = %doc_id,
        request_id,
        repo_id = %scope.repo_id,
        branch = ?scope.branch,
        scope_nonce = ?scope_nonce,
        "OpenDoc scope resolved"
    );
    let (content, base_seq, delta_ops, version) =
        match load_snapshot(state, &scope, doc_id, &repo_scope) {
            Ok(payload) => payload,
            Err(e) => {
                send_open_doc_error_with_scope_nonce(
                    ch,
                    e,
                    OpenDocErrorContext {
                        context: "Failed to load document snapshot",
                        scope_nonce,
                        repo_scope: &repo_scope,
                        doc_id,
                        request_id,
                        repo_id: scope.repo_id,
                        branch: scope.branch.as_ref(),
                    },
                );
                return;
            }
        };

    tracing::info!(
        repo_scope = repo_scope.as_str(),
        doc_id = %doc_id,
        request_id,
        repo_id = %scope.repo_id,
        branch = ?scope.branch,
        base_seq,
        version,
        pending_ops = delta_ops.len(),
        elapsed_ms = start.elapsed().as_millis(),
        "OpenDoc prepared"
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
    repo_scope: &str,
) -> anyhow::Result<SnapshotPayload> {
    state.repo.run_on_repo_db(
        &resolved_repo_type(scope.branch.as_ref(), scope.repo_id),
        |db| build_snapshot_payload(db, doc_id, state.repo.snapshot_depth, repo_scope),
    )
}

fn resolved_repo_type(branch: Option<&PeerId>, repo_id: RepoId) -> RepoType {
    match branch.cloned() {
        Some(peer_id) => RepoType::Remote(peer_id, repo_id),
        None => RepoType::Local(repo_id),
    }
}

fn resolved_repo_scope(scope: &super::ResolvedRepo) -> String {
    match scope.branch.as_ref() {
        Some(peer_id) => shadow_repo_scope(peer_id, &scope.repo_id),
        None => local_repo_scope(&scope.repo_name),
    }
}

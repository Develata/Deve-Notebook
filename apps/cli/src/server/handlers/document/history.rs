//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!
//! Document history projection handler.

use super::confirmed;
use super::errors::send_doc_error_with_scope_nonce;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::{DocId, PeerId, RepoId, RepoType};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub(super) async fn handle_request_history(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    doc_id: DocId,
    request_id: u64,
) {
    let scope_nonce = browser_scope_nonce(session);
    let Some(scope) = super::resolve_document_scope(state, ch, session, scope_nonce) else {
        return;
    };
    let ops = match load_doc_history(state, &scope, doc_id) {
        Ok(ops) => ops,
        Err(err) => {
            send_doc_error_with_scope_nonce(
                ch,
                "Failed to load document history",
                err,
                scope_nonce,
            );
            return;
        }
    };
    ch.unicast(ServerMessage::History {
        repo_id: scope.repo_id,
        branch: scope.branch.clone(),
        scope_nonce,
        doc_id,
        request_id,
        ops,
    });
}

fn browser_scope_nonce(session: &WsSession) -> Option<u64> {
    session.is_browser_session().then(|| session.scope_nonce())
}

fn load_doc_history(
    state: &Arc<AppState>,
    scope: &super::ResolvedRepo,
    doc_id: DocId,
) -> anyhow::Result<Vec<deve_core::protocol::ConfirmedOp>> {
    state.repo.run_on_repo_db(
        &resolved_repo_type(scope.branch.as_ref(), scope.repo_id),
        |db| {
            if deve_core::ledger::node_meta::file_meta_for_doc(db, doc_id)?.is_none() {
                anyhow::bail!("Document not found: {}", doc_id);
            }
            confirmed::load_doc_ops(db, doc_id)
        },
    )
}

fn resolved_repo_type(branch: Option<&PeerId>, repo_id: RepoId) -> RepoType {
    match branch.cloned() {
        Some(peer_id) => RepoType::Remote(peer_id, repo_id),
        None => RepoType::Local(repo_id),
    }
}

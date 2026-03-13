use super::confirmed;
use super::errors::send_doc_error;
use crate::server::repo_scope::{map_repo_scope_error, resolve_session_repo_and_sync};
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
    let scope = match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            ch.send_protocol_error(map_repo_scope_error(err));
            return;
        }
    };
    let ops = match load_doc_history(state, session, scope.repo_id, doc_id) {
        Ok(ops) => ops,
        Err(err) => {
            send_doc_error(ch, "Failed to load document history", err);
            return;
        }
    };
    ch.unicast(ServerMessage::History {
        repo_id: scope.repo_id,
        branch: session.active_branch.clone(),
        doc_id,
        request_id,
        ops,
    });
}

fn load_doc_history(
    state: &Arc<AppState>,
    session: &WsSession,
    repo_id: RepoId,
    doc_id: DocId,
) -> anyhow::Result<Vec<deve_core::protocol::ConfirmedOp>> {
    state.repo.run_on_repo_db(
        &resolved_repo_type(session.active_branch.as_ref(), repo_id),
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

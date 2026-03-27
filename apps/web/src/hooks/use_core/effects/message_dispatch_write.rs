use deve_core::models::{DocId, PeerId, RepoId};
use leptos::prelude::{GetUntracked, Set, Update};

use super::super::pending;
use super::super::state::CoreSignals;
use super::message_repo_scope::{accepts_write_ready_message, matches_current_message_scope};

pub fn handle_write_ready_message(
    peer_id: PeerId,
    repo_id: RepoId,
    scope_nonce: u64,
    branch: Option<PeerId>,
    ws: &crate::api::WsService,
    signals: CoreSignals,
) {
    let repo_id = repo_id.to_string();
    if !accepts_write_ready_message(&repo_id, &branch, scope_nonce, signals) {
        return;
    }
    signals.set_handshake_ready.set(true);
    ws.mark_writer_ready(repo_id, peer_id.as_str());
}

pub fn handle_ack_message(
    repo_id: RepoId,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    doc_id: DocId,
    client_op_id: u64,
    signals: CoreSignals,
) {
    if !matches_current_message_scope(&Some(repo_id), &branch, signals)
        || scope_nonce != Some(signals.current_scope_nonce.get_untracked())
    {
        return;
    }
    signals.set_pending_local_edits.update(|pending_edits| {
        let _ = pending::ack_pending_edit(pending_edits, doc_id, client_op_id);
    });
}

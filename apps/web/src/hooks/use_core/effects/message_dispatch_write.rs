//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 07_network#web-ws-runtime
//!
use deve_core::models::{DocId, PeerId, RepoId};
use leptos::prelude::{GetUntracked, Set, Update};

use super::super::state::CoreSignals;
use super::message_repo_scope::{accepts_write_ready_message, matches_current_message_scope};
use crate::runtime::document::{confirm, pending};
use deve_core::protocol::ClientMessage;

pub fn handle_write_ready_message(
    peer_id: PeerId,
    repo_id: RepoId,
    scope_nonce: u64,
    branch: Option<PeerId>,
    ws: &crate::api::WsService,
    signals: CoreSignals,
) {
    let repo_id_text = repo_id.to_string();
    if !accepts_write_ready_message(&repo_id_text, &branch, scope_nonce, signals) {
        return;
    }
    ws.mark_writer_ready(repo_id_text, scope_nonce, peer_id.as_str());
    let mut replay = None;
    signals.set_pending_document_create.update(|pending| {
        replay = pending
            .as_mut()
            .and_then(|pending| pending.take_replay_for_write_ready(repo_id, scope_nonce));
    });
    if let Some(request) = replay {
        ws.send(ClientMessage::DocumentCreate(request));
    }
}

pub fn handle_ack_message(
    repo_id: RepoId,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    doc_id: DocId,
    seq: u64,
    client_op_id: u64,
    signals: CoreSignals,
) {
    let accepts_current_scope = matches_current_message_scope(&Some(repo_id), &branch, signals)
        && scope_nonce == Some(signals.current_scope_nonce.get_untracked());
    let has_matching_pending = scope_nonce.is_some()
        && pending::has_pending_edit(
            &signals.pending_local_edits.get_untracked(),
            Some(repo_id),
            scope_nonce,
            doc_id,
            client_op_id,
        );
    if !accepts_current_scope && !has_matching_pending {
        return;
    }
    let current_doc = accepts_current_scope
        .then(|| signals.current_doc.get_untracked())
        .flatten();
    let mut clear_navigation = false;
    signals.set_pending_local_edits.update(|pending_edits| {
        clear_navigation = confirm::commit_pending_edit(
            pending_edits,
            current_doc,
            Some(repo_id),
            scope_nonce,
            doc_id,
            client_op_id,
            seq,
        )
        .clear_navigation;
    });
    if clear_navigation {
        signals.set_pending_navigation.set(None);
    }
}

#[cfg(test)]
mod tests;

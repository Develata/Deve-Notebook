use crate::hooks::use_core::apply::apply_tree_delta;
use crate::hooks::use_core::effects_msg;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::PeerId;
use deve_core::models::RepoId;
use deve_core::tree::TreeDelta;
use leptos::prelude::{GetUntracked, Set, Update};

use super::message_repo_scope::matches_current_message_scope;
use super::message_scope::accepts_system_or_matching_request;

pub fn handle_doc_list(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    docs: Vec<(deve_core::models::DocId, String)>,
    signals: CoreSignals,
) {
    if !matches_current_message_scope(&repo_id, &branch, signals)
        || !accepts_system_or_matching_request(
            request_id.as_deref(),
            signals.doc_list_request_id.get_untracked().as_deref(),
            scope_nonce,
            signals.current_scope_nonce.get_untracked(),
        )
    {
        return;
    }
    signals.set_doc_list_request_id.set(None);
    if request_id.is_none() {
        signals.set_tree_request_id.set(None);
    }
    effects_msg::handle_doc_list(docs, signals.set_docs);
}

pub fn handle_tree_update(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    delta: TreeDelta,
    signals: CoreSignals,
) {
    if !matches_current_message_scope(&repo_id, &branch, signals)
        || !accepts_system_or_matching_request(
            request_id.as_deref(),
            signals.tree_request_id.get_untracked().as_deref(),
            scope_nonce,
            signals.current_scope_nonce.get_untracked(),
        )
    {
        return;
    }
    signals.set_tree_request_id.set(None);
    if request_id.is_none() {
        signals.set_doc_list_request_id.set(None);
    }
    signals
        .set_tree_nodes
        .update(|nodes| apply_tree_delta(nodes, delta));
}

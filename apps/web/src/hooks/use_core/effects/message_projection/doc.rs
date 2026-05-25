//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::{DocId, PeerId, RepoId};
use leptos::prelude::{GetUntracked, Set};

use super::super::message_repo_scope::{
    matches_current_message_scope, matches_projection_message_scope,
};
use super::super::message_scope::accepts_system_or_matching_request;
mod selection;

pub fn handle_doc_list(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    docs: Vec<(DocId, String)>,
    signals: CoreSignals,
) {
    let matches_scope = matches_current_message_scope(&repo_id, &branch, signals);
    let matches_projection_scope = matches_projection_message_scope(&repo_id, &branch, signals)
        && scope_nonce == Some(signals.current_scope_nonce.get_untracked());
    let matches_request = accepts_system_or_matching_request(
        request_id.as_deref(),
        signals.doc_list_request_id.get_untracked().as_deref(),
        scope_nonce,
        signals.current_scope_nonce.get_untracked(),
    );
    if !(matches_scope || matches_projection_scope) || !matches_request {
        leptos::logging::log!("忽略 DocList: repo-scope 或 request gate 不匹配");
        return;
    }
    signals.set_doc_list_request_id.set(None);
    if request_id.is_none() {
        signals.set_tree_request_id.set(None);
    }
    selection::reconcile_doc_selection(&docs, signals);
    signals.set_docs.set(docs);
}

#[cfg(test)]
mod tests;

use crate::hooks::use_core::apply::apply_tree_delta;
use crate::hooks::use_core::effects_msg;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::PeerId;
use deve_core::models::RepoId;
use deve_core::tree::TreeDelta;
use leptos::prelude::{GetUntracked, Set, Update};

use super::message_repo_scope::{matches_current_message_scope, matches_projection_message_scope};
use super::message_scope::accepts_system_or_matching_request;

pub fn handle_doc_list(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    docs: Vec<(deve_core::models::DocId, String)>,
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
    leptos::logging::log!(
        "收到 DocList: request_id={:?}, repo_id={:?}, branch={:?}, scope_nonce={:?}, docs={}, matches_scope={}, matches_projection_scope={}, matches_request={}",
        request_id,
        repo_id,
        branch,
        scope_nonce,
        docs.len(),
        matches_scope,
        matches_projection_scope,
        matches_request
    );
    if !(matches_scope || matches_projection_scope) || !matches_request {
        leptos::logging::warn!("忽略 DocList: repo-scope 或 request gate 不匹配");
        return;
    }
    signals.set_doc_list_request_id.set(None);
    if request_id.is_none() {
        signals.set_tree_request_id.set(None);
    }
    effects_msg::handle_doc_list(
        docs,
        signals.current_doc,
        signals.set_current_doc,
        signals.set_docs,
    );
}

pub fn handle_tree_update(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    delta: TreeDelta,
    signals: CoreSignals,
) {
    let matches_scope = matches_current_message_scope(&repo_id, &branch, signals);
    let matches_projection_scope = matches_projection_message_scope(&repo_id, &branch, signals)
        && scope_nonce == Some(signals.current_scope_nonce.get_untracked());
    let matches_request = accepts_system_or_matching_request(
        request_id.as_deref(),
        signals.tree_request_id.get_untracked().as_deref(),
        scope_nonce,
        signals.current_scope_nonce.get_untracked(),
    );
    leptos::logging::log!(
        "收到 TreeUpdate: request_id={:?}, repo_id={:?}, branch={:?}, scope_nonce={:?}, matches_scope={}, matches_projection_scope={}, matches_request={}",
        request_id,
        repo_id,
        branch,
        scope_nonce,
        matches_scope,
        matches_projection_scope,
        matches_request
    );
    if !(matches_scope || matches_projection_scope) || !matches_request {
        leptos::logging::warn!("忽略 TreeUpdate: repo-scope 或 request gate 不匹配");
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

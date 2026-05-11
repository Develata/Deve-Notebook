//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!
use crate::hooks::use_core::apply::apply_tree_delta;
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::{PeerId, RepoId};
use deve_core::tree::TreeDelta;
use leptos::prelude::{GetUntracked, Set, Update};

use super::super::message_repo_scope::{
    matches_current_message_scope, matches_projection_message_scope,
};
use super::super::message_scope::accepts_system_or_matching_request;

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
    if !(matches_scope || matches_projection_scope) || !matches_request {
        leptos::logging::log!("忽略 TreeUpdate: repo-scope 或 request gate 不匹配");
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

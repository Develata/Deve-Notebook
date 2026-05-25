//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use leptos::prelude::GetUntracked;

use super::super::state::CoreSignals;
use deve_core::models::{PeerId, RepoId};
mod logic;

pub fn accepts_unscoped_update(signals: CoreSignals) -> bool {
    logic::accepts_unscoped_update(
        signals.pending_branch_switch.get_untracked(),
        signals.pending_repo_switch.get_untracked(),
    )
}

pub fn accepts_plugin_response(req_id: &str, signals: CoreSignals) -> bool {
    accepts_unscoped_update(signals)
        && logic::contains_request_id(req_id, &signals.plugin_request_ids.get_untracked())
}

pub fn accepts_chat_chunk(req_id: &str, signals: CoreSignals) -> bool {
    accepts_unscoped_update(signals)
        && (logic::contains_request_id(req_id, &signals.plugin_request_ids.get_untracked())
            || logic::contains_chat_message(req_id, &signals.chat_messages.get_untracked()))
}

pub fn accepts_search_results(
    request_id: &str,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    signals: CoreSignals,
) -> bool {
    accepts_unscoped_update(signals)
        && scope_nonce == Some(signals.current_scope_nonce.get_untracked())
        && repo_id.map(|id| id.to_string()) == signals.current_repo_id.get_untracked()
        && branch == signals.active_branch.get_untracked()
        && signals.search_request_id.get_untracked().as_deref() == Some(request_id)
}

#[cfg(test)]
mod tests;

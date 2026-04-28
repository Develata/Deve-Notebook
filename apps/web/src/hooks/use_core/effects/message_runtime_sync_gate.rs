//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::{PeerId, RepoId};
use leptos::prelude::{GetUntracked, ReadSignal, Set};

use super::super::message_repo_scope::matches_current_message_scope;
use super::super::message_scope::accepts_system_or_matching_request;

pub fn accepts_runtime_message(
    repo_id: &Option<RepoId>,
    branch: &Option<PeerId>,
    scope_nonce: Option<u64>,
    signals: CoreSignals,
) -> bool {
    matches_current_message_scope(repo_id, branch, signals)
        && accepts_system_or_matching_request(
            None,
            None,
            scope_nonce,
            signals.current_scope_nonce.get_untracked(),
        )
}

pub fn accepts_runtime_request(
    request_id: Option<&str>,
    tracked_request_id: ReadSignal<Option<String>>,
    repo_id: &Option<RepoId>,
    branch: &Option<PeerId>,
    scope_nonce: Option<u64>,
    signals: CoreSignals,
) -> bool {
    matches_current_message_scope(repo_id, branch, signals)
        && accepts_system_or_matching_request(
            request_id,
            tracked_request_id.get_untracked().as_deref(),
            scope_nonce,
            signals.current_scope_nonce.get_untracked(),
        )
}

pub fn clear_pending_ops(signals: CoreSignals) {
    signals.set_pending_ops_count.set(0);
    signals.set_pending_ops_previews.set(vec![]);
}

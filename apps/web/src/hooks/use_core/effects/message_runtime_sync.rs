use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::{PeerId, RepoId};
use leptos::prelude::{GetUntracked, Set};

use super::message_repo_scope::matches_current_message_scope;
use super::message_scope::accepts_system_or_matching_request;

fn accepts_runtime_message(
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

pub fn handle_sync_mode_status(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    mode: String,
    signals: CoreSignals,
) {
    if !matches_current_message_scope(&repo_id, &branch, signals)
        || !accepts_system_or_matching_request(
            request_id.as_deref(),
            signals.sync_mode_request_id.get_untracked().as_deref(),
            scope_nonce,
            signals.current_scope_nonce.get_untracked(),
        )
    {
        return;
    }
    signals.set_sync_mode_request_id.set(None);
    signals.set_sync_mode.set(mode);
}

pub fn handle_pending_ops_info(
    request_id: Option<String>,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    count: u32,
    previews: Vec<(String, String, String)>,
    signals: CoreSignals,
) {
    if !matches_current_message_scope(&repo_id, &branch, signals)
        || !accepts_system_or_matching_request(
            request_id.as_deref(),
            signals.pending_ops_request_id.get_untracked().as_deref(),
            scope_nonce,
            signals.current_scope_nonce.get_untracked(),
        )
    {
        return;
    }
    signals.set_pending_ops_request_id.set(None);
    signals.set_pending_ops_count.set(count);
    signals.set_pending_ops_previews.set(previews);
}

pub fn handle_merge_complete(
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    merged_count: u32,
    signals: CoreSignals,
) {
    if !accepts_runtime_message(&repo_id, &branch, scope_nonce, signals) {
        return;
    }
    leptos::logging::log!("已合并 {} 个操作", merged_count);
    signals.set_pending_ops_count.set(0);
    signals.set_pending_ops_previews.set(vec![]);
}

pub fn handle_pending_discarded(
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    signals: CoreSignals,
) {
    if !accepts_runtime_message(&repo_id, &branch, scope_nonce, signals) {
        return;
    }
    leptos::logging::log!("待处理操作已丢弃");
    signals.set_pending_ops_count.set(0);
    signals.set_pending_ops_previews.set(vec![]);
}

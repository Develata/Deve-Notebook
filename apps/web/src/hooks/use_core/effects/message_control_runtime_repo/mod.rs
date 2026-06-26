//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::{LoadPhase, SyncModeState};
#[cfg(test)]
use deve_core::models::PeerId;
use leptos::prelude::Set;

use super::super::effects_sc_state::{self, clear_repo_scoped_state};
use super::super::state::CoreSignals;
mod requests;

pub(super) fn clear_repo_scoped_runtime(signals: CoreSignals) {
    signals.set_peers.set(Default::default());
    signals.set_load_state.set(LoadPhase::Ready);
    signals.set_load_progress.set((0, 0));
    signals.set_load_eta_ms.set(0);
    signals.set_sync_mode.set(SyncModeState::Auto);
    signals.set_sync_mode_request_id.set(None);
    signals.set_pending_ops_count.set(0);
    signals.set_pending_ops_previews.set(Vec::new());
    signals.set_pending_ops_request_id.set(None);
    signals.set_handshake_scope_nonce.set(None);
    signals.set_pending_branch_switch.set(None);
    signals.set_pending_repo_switch.set(None);
    signals.set_plugin_response.set(None);
    signals.set_plugin_request_ids.set(Vec::new());
    signals.set_chat_messages.set(Vec::new());
    signals.set_is_chat_streaming.set(false);
    signals.set_shadow_list_request_id.set(None);
    signals.set_repo_list_request_id.set(None);
    signals.set_repo_entries.set(Vec::new());
    signals.set_doc_list_request_id.set(None);
    signals.set_tree_request_id.set(None);
    signals.set_search_request_id.set(None);
    signals.set_changes_request_id.set(None);
    signals.set_commit_history_request_id.set(None);
    signals.set_doc_diff_request_id.set(None);
    signals.set_commit_diff_request_id.set(None);
    signals.set_search_results.set(Vec::new());
    clear_repo_scoped_state(effects_sc_state::ScStateResetSignals {
        set_staged: signals.set_staged_changes,
        set_unstaged: signals.set_unstaged_changes,
        set_confirmed: signals.set_confirmed_changes,
        set_changes_request_id: signals.set_changes_request_id,
        set_history: signals.set_commit_history,
        set_commit_history_request_id: signals.set_commit_history_request_id,
        set_doc_diff_request_id: signals.set_doc_diff_request_id,
        set_diff: signals.set_diff_content,
        set_commit_diff_request_id: signals.set_commit_diff_request_id,
        set_commit_diff: signals.set_commit_diff_result,
        set_notice: signals.set_source_control_notice,
    });
}

pub(super) fn request_repo_sync_state(ws: &WsService, signals: CoreSignals) {
    requests::request_repo_sync_state(ws, signals);
}

pub(super) fn request_repo_list(ws: &WsService, signals: CoreSignals) {
    requests::request_repo_list(ws, signals);
}

#[cfg(test)]
pub(super) fn next_request_id() -> String {
    requests::next_request_id()
}

#[cfg(test)]
pub(super) fn should_request_repo_sync_state(active_branch: Option<PeerId>) -> bool {
    requests::should_request_repo_sync_state(active_branch)
}

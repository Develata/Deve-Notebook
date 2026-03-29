use crate::api::WsService;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Set};

use super::super::effects_sc;
use super::super::state::CoreSignals;

pub(super) fn clear_repo_scoped_runtime(signals: CoreSignals) {
    signals.set_peers.set(Default::default());
    signals.set_load_state.set("ready".to_string());
    signals.set_load_progress.set((0, 0));
    signals.set_load_eta_ms.set(0);
    signals.set_sync_mode.set("auto".to_string());
    signals.set_sync_mode_request_id.set(None);
    signals.set_pending_ops_count.set(0);
    signals.set_pending_ops_previews.set(Vec::new());
    signals.set_pending_ops_request_id.set(None);
    signals.set_handshake_scope_nonce.set(None);
    signals.set_pending_branch_switch_nonce.set(None);
    signals.set_pending_repo_switch_nonce.set(None);
    signals.set_pending_local_edits.set(Default::default());
    signals.set_plugin_response.set(None);
    signals.set_plugin_request_ids.set(Vec::new());
    signals.set_chat_messages.set(Vec::new());
    signals.set_is_chat_streaming.set(false);
    signals.set_shadow_list_request_id.set(None);
    signals.set_repo_list_request_id.set(None);
    signals.set_doc_list_request_id.set(None);
    signals.set_tree_request_id.set(None);
    signals.set_search_request_id.set(None);
    signals.set_changes_request_id.set(None);
    signals.set_commit_history_request_id.set(None);
    signals.set_doc_diff_request_id.set(None);
    signals.set_commit_diff_request_id.set(None);
    signals.set_search_results.set(Vec::new());
    effects_sc::clear_repo_scoped_state(super::super::effects_sc_state::ScStateResetSignals {
        set_staged: signals.set_staged_changes,
        set_unstaged: signals.set_unstaged_changes,
        set_changes_request_id: signals.set_changes_request_id,
        set_history: signals.set_commit_history,
        set_commit_history_request_id: signals.set_commit_history_request_id,
        set_doc_diff_request_id: signals.set_doc_diff_request_id,
        set_diff: signals.set_diff_content,
        set_commit_diff_request_id: signals.set_commit_diff_request_id,
        set_commit_diff: signals.set_commit_diff_result,
    });
}

pub(super) fn request_repo_sync_state(ws: &WsService, signals: CoreSignals) {
    if !should_request_repo_sync_state(signals.active_branch.get_untracked()) {
        return;
    }
    let scope_nonce = signals.current_scope_nonce.get_untracked();
    let sync_mode_request_id = next_request_id();
    let pending_ops_request_id = next_request_id();
    signals
        .set_sync_mode_request_id
        .set(Some(sync_mode_request_id.clone()));
    signals
        .set_pending_ops_request_id
        .set(Some(pending_ops_request_id.clone()));
    ws.send(ClientMessage::GetSyncMode {
        request_id: sync_mode_request_id,
        scope_nonce: Some(scope_nonce),
    });
    ws.send(ClientMessage::GetPendingOps {
        request_id: pending_ops_request_id,
        scope_nonce: Some(scope_nonce),
    });
}

pub(super) fn request_repo_list(ws: &WsService, signals: CoreSignals) {
    let request_id = next_request_id();
    signals
        .set_repo_list_request_id
        .set(Some(request_id.clone()));
    ws.send(ClientMessage::ListRepos {
        request_id,
        scope_nonce: Some(signals.current_scope_nonce.get_untracked()),
    });
}

pub(super) fn next_request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub(super) fn should_request_repo_sync_state(active_branch: Option<PeerId>) -> bool {
    active_branch.is_none()
}

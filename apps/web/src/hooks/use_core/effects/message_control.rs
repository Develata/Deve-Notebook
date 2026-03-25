use crate::api::WsService;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use leptos::prelude::{GetUntracked, Set};

use super::super::effects_sc;
use super::super::effects_switch;
use super::super::state::CoreSignals;
use super::message_scope::string_branch_matches_scope;
use super::message_shadow;

pub fn handle_branch_switched(
    peer_id: Option<String>,
    success: bool,
    switch_nonce: Option<u64>,
    ws: &WsService,
    signals: CoreSignals,
) {
    if effects_switch::handle_branch_switched(
        peer_id,
        success,
        switch_nonce,
        effects_switch::BranchSwitchSignals {
            pending_branch_switch: signals.pending_branch_switch,
            pending_branch_switch_nonce: signals.pending_branch_switch_nonce,
            set_pending_branch_switch: signals.set_pending_branch_switch,
            set_pending_branch_switch_nonce: signals.set_pending_branch_switch_nonce,
            set_active_branch: signals.set_active_branch,
        },
    ) {
        if let Some(switch_nonce) = switch_nonce {
            signals.set_current_scope_nonce.set(switch_nonce);
        }
        ws.clear_writer_ready();
        signals.set_handshake_ready.set(false);
        signals.set_pending_repo_switch.set(None);
        signals.set_current_repo.set(None);
        signals.set_current_repo_id.set(None);
        signals.set_current_doc.set(None);
        signals.set_docs.set(Vec::new());
        signals.set_tree_nodes.set(Vec::new());
        signals.set_repo_list.set(Vec::new());
        clear_repo_scoped_runtime(signals);
        request_repo_list(ws, signals);
    }
}

pub fn handle_repo_switched(
    branch: Option<String>,
    name: String,
    uuid: String,
    switch_nonce: Option<u64>,
    ws: &WsService,
    signals: CoreSignals,
) {
    leptos::logging::log!(
        "收到 RepoSwitched: branch={:?}, name={}, uuid={}, switch_nonce={:?}",
        branch,
        name,
        uuid,
        switch_nonce
    );
    if !string_branch_matches_scope(
        &branch,
        signals.active_branch.get_untracked(),
        signals.pending_branch_switch.get_untracked(),
    ) {
        leptos::logging::warn!("忽略 RepoSwitched: branch 与当前 scope 不匹配");
        return;
    }
    let outcome = effects_switch::handle_repo_switched(
        name,
        uuid,
        switch_nonce,
        crate::hooks::use_core::RepoSwitchSignals {
            current_repo: signals.current_repo,
            current_repo_id: signals.current_repo_id,
            pending_repo_switch: signals.pending_repo_switch,
            set_pending_repo_switch: signals.set_pending_repo_switch,
            pending_repo_switch_nonce: signals.pending_repo_switch_nonce,
            set_pending_repo_switch_nonce: signals.set_pending_repo_switch_nonce,
            current_scope_nonce: signals.current_scope_nonce,
            set_current_scope_nonce: signals.set_current_scope_nonce,
            set_current_repo: signals.set_current_repo,
            set_current_repo_id: signals.set_current_repo_id,
            set_current_doc: signals.set_current_doc,
        },
    );
    leptos::logging::log!(
        "处理 RepoSwitched 结果: accepted={}, should_refresh={}, current_repo={:?}, current_repo_id={:?}, scope_nonce={}",
        outcome.accepted,
        outcome.should_refresh,
        signals.current_repo.get_untracked(),
        signals.current_repo_id.get_untracked(),
        signals.current_scope_nonce.get_untracked()
    );
    if outcome.should_refresh {
        ws.clear_writer_ready();
        signals.set_handshake_ready.set(false);
        signals.set_docs.set(Vec::new());
        signals.set_tree_nodes.set(Vec::new());
        clear_repo_scoped_runtime(signals);
        request_repo_list(ws, signals);
        request_repo_sync_state(ws, signals);
        message_shadow::request_shadow_list(ws, signals);
    }
}

fn clear_repo_scoped_runtime(signals: CoreSignals) {
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

fn request_repo_sync_state(ws: &WsService, signals: CoreSignals) {
    if !should_request_repo_sync_state(signals.active_branch.get_untracked()) {
        return;
    }
    let scope_nonce = signals.current_scope_nonce.get_untracked();
    let sync_mode_request_id = uuid::Uuid::new_v4().to_string();
    let pending_ops_request_id = uuid::Uuid::new_v4().to_string();
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

fn request_repo_list(ws: &WsService, signals: CoreSignals) {
    let request_id = next_request_id();
    signals
        .set_repo_list_request_id
        .set(Some(request_id.clone()));
    ws.send(ClientMessage::ListRepos {
        request_id,
        scope_nonce: Some(signals.current_scope_nonce.get_untracked()),
    });
}

fn next_request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn should_request_repo_sync_state(active_branch: Option<PeerId>) -> bool {
    active_branch.is_none()
}

#[cfg(test)]
mod tests {
    use super::{next_request_id, should_request_repo_sync_state};
    use deve_core::protocol::ClientMessage;

    #[test]
    fn repo_sync_state_requests_only_run_on_local_branch() {
        assert!(should_request_repo_sync_state(None));
        assert!(!should_request_repo_sync_state(Some(
            deve_core::models::PeerId::new("peer-a")
        )));
    }

    #[test]
    fn request_ids_are_non_empty() {
        let request_id = next_request_id();
        assert!(!request_id.is_empty());
        assert!(uuid::Uuid::parse_str(&request_id).is_ok());
    }

    #[test]
    fn list_repos_request_keeps_shared_request_id_shape() {
        let request_id = next_request_id();
        let msg = ClientMessage::ListRepos {
            request_id: request_id.clone(),
            scope_nonce: Some(7),
        };
        assert!(matches!(
            msg,
            ClientMessage::ListRepos {
                request_id: actual,
                scope_nonce: Some(7),
            } if actual == request_id
        ));
    }
}

//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::write_gate::RepoWriteGateState;
use leptos::prelude::*;

use super::gate::source_control_refresh_allowed;

pub(in crate::hooks::use_core) struct CommitRefreshSignals {
    pub expected_scope_nonce: u64,
    pub current_scope_nonce: ReadSignal<u64>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub load_state: ReadSignal<String>,
    pub is_spectator: Signal<bool>,
    pub handshake_ready: ReadSignal<bool>,
    pub pending_branch_switch: ReadSignal<Option<super::super::types::PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
    pub set_changes_request_id: WriteSignal<Option<String>>,
    pub set_commit_history_request_id: WriteSignal<Option<String>>,
}

pub(in crate::hooks::use_core) fn refresh_after_commit(
    commit_id: &str,
    signals: CommitRefreshSignals,
    ws: &WsService,
) {
    leptos::logging::log!("已提交: {}", commit_id);
    let current_scope_nonce = signals.current_scope_nonce.get_untracked();
    let repo_id = signals.current_repo_id.get_untracked();
    let load_state = signals.load_state.get_untracked();
    let pending_branch_switch = signals.pending_branch_switch.get_untracked();
    let pending_repo_switch = signals.pending_repo_switch.get_untracked();
    let readiness = ws.native_runtime_readiness_for_untracked(
        repo_id.as_deref(),
        Some(current_scope_nonce),
        signals.handshake_ready.get_untracked(),
    );
    if !source_control_refresh_allowed(
        signals.expected_scope_nonce,
        current_scope_nonce,
        RepoWriteGateState {
            connection_status: ws.status.get_untracked(),
            load_state: &load_state,
            is_read_only: signals.is_spectator.get_untracked(),
            node_role_probe_failed: ws.node_role_probe_failed.get_untracked(),
            node_role_readable: readiness.node_role_readable,
            handshake_ready: readiness.repo_handshake_complete,
            writer_ready: readiness.writer_ready,
            has_repo: repo_id.is_some(),
            pending_branch_switch: pending_branch_switch.is_some(),
            pending_repo_switch: pending_repo_switch.is_some(),
        },
    ) {
        return;
    }
    let changes_request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_changes_request_id
        .set(Some(changes_request_id.clone()));
    ws.send(deve_core::protocol::ClientMessage::GetChanges {
        request_id: changes_request_id,
        scope_nonce: Some(current_scope_nonce),
    });
    let history_request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_commit_history_request_id
        .set(Some(history_request_id.clone()));
    ws.send(deve_core::protocol::ClientMessage::GetCommitHistory {
        request_id: history_request_id,
        limit: 50,
        scope_nonce: Some(current_scope_nonce),
    });
}

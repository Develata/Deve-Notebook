//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::write_gate::RepoWriteGateState;
use crate::hooks::use_core::{LoadPhase, PendingBranchSwitch, PendingRepoSwitch};
use crate::storage::DegradedSyncMode;
use leptos::prelude::*;

use super::super::effects_sc_feedback::show_file_op_feedback;
use super::gate::source_control_refresh_allowed;

pub(in crate::hooks::use_core) struct FsRefreshSignals {
    pub expected_scope_nonce: u64,
    pub current_scope_nonce: ReadSignal<u64>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub load_state: ReadSignal<LoadPhase>,
    pub is_spectator: Signal<bool>,
    pub handshake_ready: ReadSignal<bool>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pub pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
    pub degraded_sync_mode: ReadSignal<Option<DegradedSyncMode>>,
    pub sync_banner: ReadSignal<Option<String>>,
    pub set_sync_banner: WriteSignal<Option<String>>,
    pub set_doc_list_request_id: WriteSignal<Option<String>>,
    pub set_tree_request_id: WriteSignal<Option<String>>,
}

pub(in crate::hooks::use_core) fn refresh_after_fs_change(
    path: &str,
    change_type: &str,
    has_conflict: bool,
    signals: FsRefreshSignals,
    schedule_refresh: &dyn Fn(),
    ws: &WsService,
) {
    let conflict_tag = if has_conflict { " [冲突]" } else { "" };
    if has_conflict || change_type != "dir_changed" {
        leptos::logging::log!("文件变更: {} ({}){}", path, change_type, conflict_tag);
    }
    show_file_op_feedback(
        path,
        change_type,
        has_conflict,
        signals.degraded_sync_mode,
        signals.sync_banner,
        signals.set_sync_banner,
    );
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
            load_state: load_state.as_str(),
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
    schedule_refresh();
    let request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_doc_list_request_id
        .set(Some(request_id.clone()));
    signals.set_tree_request_id.set(Some(request_id.clone()));
    ws.send(deve_core::protocol::ClientMessage::ListDocs {
        request_id,
        scope_nonce: Some(current_scope_nonce),
    });
}

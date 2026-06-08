//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use super::super::contexts::SourceControlContext;
use super::super::types::CoreState;
use super::super::write_gate::{
    RepoWriteSignals, repo_source_control_read_block_tracked, repo_write_allowed_for_core_tracked,
    repo_write_block_tracked,
};
use leptos::prelude::{Callback, Set, Signal};

pub(super) fn build_source_control_context(state: &CoreState) -> SourceControlContext {
    let source_control = &state.runtime_clients.source_control;
    let scope = &state.runtime_clients.scope;
    let state_for_can_write = state.clone();
    let state_for_block = state.clone();
    let state_for_read_block = state.clone();
    let clear_notice = Callback::new({
        let set_notice = state.set_source_control_notice;
        move |_| set_notice.set(None)
    });

    SourceControlContext {
        staged_changes: source_control.staged_changes,
        unstaged_changes: source_control.unstaged_changes,
        commit_history: source_control.commit_history,
        commit_history_request_id: source_control.commit_history_request_id,
        commit_diff_request_id: source_control.commit_diff_request_id,
        set_commit_diff_request_id: source_control.set_commit_diff_request_id,
        can_write: Signal::derive(move || {
            repo_write_allowed_for_core_tracked(&state_for_can_write)
        }),
        write_block: Signal::derive(move || {
            repo_write_block_tracked(
                &state_for_block.ws,
                RepoWriteSignals {
                    load_state: state_for_block.load_state,
                    is_spectator: state_for_block.is_spectator,
                    handshake_ready: state_for_block.handshake_ready,
                    current_repo_id: state_for_block.current_repo_id,
                    current_scope_nonce: state_for_block.current_scope_nonce,
                    active_branch: state_for_block.active_branch,
                    pending_branch_switch: state_for_block.pending_branch_switch,
                    pending_repo_switch: state_for_block.pending_repo_switch,
                },
            )
        }),
        read_block: Signal::derive(move || {
            repo_source_control_read_block_tracked(
                &state_for_read_block.ws,
                RepoWriteSignals {
                    load_state: state_for_read_block.load_state,
                    is_spectator: state_for_read_block.is_spectator,
                    handshake_ready: state_for_read_block.handshake_ready,
                    current_repo_id: state_for_read_block.current_repo_id,
                    current_scope_nonce: state_for_read_block.current_scope_nonce,
                    active_branch: state_for_read_block.active_branch,
                    pending_branch_switch: state_for_read_block.pending_branch_switch,
                    pending_repo_switch: state_for_read_block.pending_repo_switch,
                },
            )
        }),
        notice: state.source_control_notice,
        set_notice: state.set_source_control_notice,
        clear_notice,
        current_repo_id: scope.current_repo_id,
        current_scope_nonce: scope.current_scope_nonce,
        active_branch: scope.active_branch,
        pending_branch_switch: state.pending_branch_switch,
        pending_repo_switch: scope.pending_repo_switch,
        on_get_changes: source_control.on_get_changes,
        on_stage_file: source_control.on_stage_file,
        on_stage_files: source_control.on_stage_files,
        on_unstage_file: source_control.on_unstage_file,
        on_unstage_files: source_control.on_unstage_files,
        on_discard_file: source_control.on_discard_file,
        on_discard_pending: state.on_discard_pending,
        on_commit: source_control.on_commit,
        on_get_history: source_control.on_get_history,
        diff_content: source_control.diff_content,
        set_diff_content: source_control.set_diff_content,
        on_get_doc_diff: source_control.on_get_doc_diff,
        commit_diff_result: source_control.commit_diff_result,
        set_commit_diff_result: source_control.set_commit_diff_result,
        on_resolve_conflict: source_control.on_resolve_conflict,
        on_get_commit_diff: source_control.on_get_commit_diff,
        on_commit_and_push: source_control.on_commit_and_push,
    }
}

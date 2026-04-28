//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-scope-runtime
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use super::super::contexts::SourceControlContext;
use super::super::types::CoreState;
use super::super::write_gate::{
    RepoWriteSignals, repo_source_control_read_block_tracked, repo_write_allowed_for_core_tracked,
    repo_write_block_tracked,
};
use leptos::prelude::{Callback, Set, Signal};

pub(super) fn build_source_control_context(state: &CoreState) -> SourceControlContext {
    let state_for_can_write = state.clone();
    let state_for_block = state.clone();
    let state_for_read_block = state.clone();
    let clear_notice = Callback::new({
        let set_notice = state.set_source_control_notice;
        move |_| set_notice.set(None)
    });

    SourceControlContext {
        staged_changes: state.staged_changes,
        unstaged_changes: state.unstaged_changes,
        commit_history: state.commit_history,
        commit_history_request_id: state.commit_history_request_id,
        commit_diff_request_id: state.commit_diff_request_id,
        set_commit_diff_request_id: state.set_commit_diff_request_id,
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
        current_repo_id: state.current_repo_id,
        active_branch: state.active_branch,
        pending_branch_switch: state.pending_branch_switch,
        pending_repo_switch: state.pending_repo_switch,
        on_get_changes: state.on_get_changes,
        on_stage_file: state.on_stage_file,
        on_stage_files: state.on_stage_files,
        on_unstage_file: state.on_unstage_file,
        on_unstage_files: state.on_unstage_files,
        on_discard_file: state.on_discard_file,
        on_discard_pending: state.on_discard_pending,
        on_commit: state.on_commit,
        on_get_history: state.on_get_history,
        diff_content: state.diff_content,
        set_diff_content: state.set_diff_content,
        on_get_doc_diff: state.on_get_doc_diff,
        commit_diff_result: state.commit_diff_result,
        set_commit_diff_result: state.set_commit_diff_result,
        on_resolve_conflict: state.on_resolve_conflict,
        on_get_commit_diff: state.on_get_commit_diff,
        on_commit_and_push: state.on_commit_and_push,
    }
}

//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#external-changes-sibling-view
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use super::super::contexts::ExternalChangesContext;
use super::super::types::CoreState;
use super::super::write_gate::{
    RepoWriteSignals, repo_source_control_read_block_tracked, repo_write_allowed_for_core_tracked,
    repo_write_block_tracked,
};
use leptos::prelude::Signal;

pub(super) fn build_external_changes_context(state: &CoreState) -> ExternalChangesContext {
    let external_changes = &state.runtime_clients.external_changes;
    let scope = &state.runtime_clients.scope;
    let state_for_can_write = state.clone();
    let state_for_block = state.clone();
    let state_for_read_block = state.clone();

    ExternalChangesContext {
        staged_changes: external_changes.staged_changes,
        unstaged_changes: external_changes.unstaged_changes,
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
        current_repo_id: scope.current_repo_id,
        current_scope_nonce: scope.current_scope_nonce,
        active_branch: scope.active_branch,
        pending_branch_switch: state.pending_branch_switch,
        pending_repo_switch: scope.pending_repo_switch,
        on_get_changes: external_changes.on_get_changes,
        on_stage_file: external_changes.on_stage_file,
        on_stage_files: external_changes.on_stage_files,
        on_unstage_file: external_changes.on_unstage_file,
        on_unstage_files: external_changes.on_unstage_files,
        on_discard_file: external_changes.on_discard_file,
        on_apply_to_ledger: external_changes.on_apply_to_ledger,
        on_get_doc_diff: external_changes.on_get_doc_diff,
    }
}

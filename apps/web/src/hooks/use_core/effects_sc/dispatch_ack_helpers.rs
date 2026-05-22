//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use leptos::prelude::Set;

use super::super::effects_sc_apply::{
    CommitRefreshSignals, FsRefreshSignals, refresh_after_commit, refresh_after_fs_change,
};
use super::super::effects_sc_feedback::{
    commit_ack_message, show_sc_ack_feedback, source_control_ack_message,
};
use super::ScMessageContext;

pub(super) fn handle_simple_ack(
    ctx: &ScMessageContext<'_>,
    log_action: &str,
    banner_action: &str,
    path: &str,
) {
    ctx.set_notice.set(None);
    leptos::logging::log!("{}: {}", log_action, path);
    show_ack_feedback(ctx, source_control_ack_message(banner_action, path));
    ctx.schedule_refresh();
}

pub(super) fn handle_commit_ack(
    ctx: &ScMessageContext<'_>,
    commit_id: &str,
    active_scope_nonce: u64,
) {
    ctx.set_notice.set(None);
    ctx.set_diff.set(None);
    ctx.set_commit_diff.set(Vec::new());
    show_ack_feedback(ctx, commit_ack_message(commit_id));
    refresh_after_commit(
        commit_id,
        CommitRefreshSignals {
            expected_scope_nonce: active_scope_nonce,
            current_scope_nonce: ctx.current_scope_nonce,
            current_repo_id: ctx.current_repo_id,
            load_state: ctx.load_state,
            is_spectator: ctx.is_spectator,
            handshake_ready: ctx.handshake_ready,
            pending_branch_switch: ctx.pending_branch_switch,
            pending_repo_switch: ctx.pending_repo_switch,
            set_changes_request_id: ctx.set_changes_request_id,
            set_commit_history_request_id: ctx.set_commit_history_request_id,
            set_doc_list_request_id: ctx.set_doc_list_request_id,
            set_tree_request_id: ctx.set_tree_request_id,
        },
        ctx.ws,
    );
}

pub(super) fn handle_fs_change_ack(
    ctx: &ScMessageContext<'_>,
    active_scope_nonce: u64,
    path: &str,
    change_type: &str,
    has_conflict: bool,
) {
    refresh_after_fs_change(
        path,
        change_type,
        has_conflict,
        FsRefreshSignals {
            expected_scope_nonce: active_scope_nonce,
            current_scope_nonce: ctx.current_scope_nonce,
            current_repo_id: ctx.current_repo_id,
            load_state: ctx.load_state,
            is_spectator: ctx.is_spectator,
            handshake_ready: ctx.handshake_ready,
            pending_branch_switch: ctx.pending_branch_switch,
            pending_repo_switch: ctx.pending_repo_switch,
            degraded_sync_mode: ctx.degraded_sync_mode,
            sync_banner: ctx.sync_banner,
            set_sync_banner: ctx.set_sync_banner,
            set_doc_list_request_id: ctx.set_doc_list_request_id,
            set_tree_request_id: ctx.set_tree_request_id,
        },
        ctx.schedule_refresh,
        ctx.ws,
    );
}

pub(super) fn handle_conflict_resolved_ack(
    ctx: &ScMessageContext<'_>,
    path: &str,
    resolution: &str,
) {
    ctx.set_notice.set(None);
    leptos::logging::log!("冲突已解决: {} ({})", path, resolution);
    show_ack_feedback(ctx, source_control_ack_message("Resolved conflict", path));
    ctx.schedule_refresh();
}

fn show_ack_feedback(ctx: &ScMessageContext<'_>, message: String) {
    show_sc_ack_feedback(
        message,
        ctx.degraded_sync_mode,
        ctx.sync_banner,
        ctx.set_sync_banner,
    );
}

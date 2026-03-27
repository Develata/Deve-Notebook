use deve_core::protocol::ServerMessage;

use super::super::effects_sc_apply::{
    FsRefreshSignals, refresh_after_commit, refresh_after_fs_change,
};
use super::ScMessageContext;

pub(crate) fn handle_sc_ack_message(
    msg: &ServerMessage,
    ctx: &ScMessageContext<'_>,
    active_scope_nonce: u64,
) -> bool {
    match msg {
        ServerMessage::StageAck {
            repo_id,
            branch,
            scope_nonce,
            path,
        } => {
            if !ctx.in_ack_scope(repo_id, branch, *scope_nonce) {
                return true;
            }
            leptos::logging::log!("已暂存: {}", path);
            ctx.schedule_refresh();
            true
        }
        ServerMessage::UnstageAck {
            repo_id,
            branch,
            scope_nonce,
            path,
        } => {
            if !ctx.in_ack_scope(repo_id, branch, *scope_nonce) {
                return true;
            }
            leptos::logging::log!("已取消暂存: {}", path);
            ctx.schedule_refresh();
            true
        }
        ServerMessage::DiscardAck {
            repo_id,
            branch,
            scope_nonce,
            path,
        } => {
            if !ctx.in_ack_scope(repo_id, branch, *scope_nonce) {
                return true;
            }
            leptos::logging::log!("已放弃变更: {}", path);
            ctx.schedule_refresh();
            true
        }
        ServerMessage::CommitAck {
            commit_id,
            repo_id,
            branch,
            scope_nonce,
            ..
        } => {
            if !ctx.in_ack_scope(repo_id, branch, *scope_nonce) {
                return true;
            }
            refresh_after_commit(
                commit_id,
                active_scope_nonce,
                ctx.set_changes_request_id,
                ctx.set_commit_history_request_id,
                ctx.ws,
            );
            true
        }
        ServerMessage::FsChangeDetected {
            repo_id,
            branch,
            scope_nonce,
            path,
            change_type,
            has_conflict,
        } => {
            if !ctx.in_ack_scope(repo_id, branch, *scope_nonce) {
                return true;
            }
            refresh_after_fs_change(
                path,
                change_type,
                *has_conflict,
                FsRefreshSignals {
                    current_scope_nonce: active_scope_nonce,
                    set_doc_list_request_id: ctx.set_doc_list_request_id,
                    set_tree_request_id: ctx.set_tree_request_id,
                },
                ctx.schedule_refresh,
                ctx.ws,
            );
            true
        }
        ServerMessage::ConflictResolved {
            repo_id,
            branch,
            scope_nonce,
            path,
            resolution,
        } => {
            if !ctx.in_ack_scope(repo_id, branch, *scope_nonce) {
                return true;
            }
            leptos::logging::log!("冲突已解决: {} ({})", path, resolution);
            ctx.schedule_refresh();
            true
        }
        _ => false,
    }
}

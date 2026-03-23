use deve_core::protocol::ServerMessage;
use leptos::prelude::{GetUntracked, Set};

use super::super::effects_sc_apply::{
    FsRefreshSignals, apply_doc_diff, refresh_after_commit, refresh_after_fs_change,
};
use super::super::effects_sc_state::{
    changes_list_matches_request, commit_diff_matches_request, commit_history_matches_request,
    doc_diff_matches_request,
};
use super::ScMessageContext;

pub(crate) fn handle_sc_message(msg: &ServerMessage, ctx: &ScMessageContext<'_>) -> bool {
    let active_scope_nonce = ctx.active_scope_nonce();
    match msg {
        ServerMessage::ChangesList {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            staged,
            unstaged,
        } => {
            if !ctx.in_scope(repo_id, branch) {
                return true;
            }
            if !changes_list_matches_request(
                request_id,
                ctx.changes_request_id.get_untracked(),
                *scope_nonce,
                active_scope_nonce,
            ) {
                return true;
            }
            ctx.set_changes_request_id.set(None);
            ctx.set_staged.set(staged.clone());
            ctx.set_unstaged.set(unstaged.clone());
        }
        ServerMessage::CommitHistory {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            commits,
        } => {
            if !ctx.in_scope(repo_id, branch) {
                return true;
            }
            if !commit_history_matches_request(
                request_id,
                ctx.commit_history_request_id.get_untracked(),
                *scope_nonce,
                active_scope_nonce,
            ) {
                return true;
            }
            ctx.set_commit_history_request_id.set(None);
            ctx.set_history.set(commits.clone());
        }
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
        }
        ServerMessage::DocDiff {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            path,
            old_content,
            new_content,
        } => {
            if !ctx.in_scope(repo_id, branch) {
                return true;
            }
            if !doc_diff_matches_request(
                request_id,
                ctx.doc_diff_request_id.get_untracked(),
                *scope_nonce,
                active_scope_nonce,
            ) {
                return true;
            }
            ctx.set_doc_diff_request_id.set(None);
            apply_doc_diff(path, old_content, new_content, ctx.set_diff);
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
        }
        ServerMessage::CommitDiffResult {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            diffs,
        } => {
            if !ctx.in_scope(repo_id, branch) {
                return true;
            }
            if !commit_diff_matches_request(
                request_id,
                ctx.commit_diff_request_id.get_untracked(),
                *scope_nonce,
                active_scope_nonce,
            ) {
                return true;
            }
            ctx.set_commit_diff_request_id.set(None);
            leptos::logging::log!("收到提交差异: {} 个文件变更", diffs.len());
            ctx.set_commit_diff.set(diffs.clone());
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
        }
        _ => return false,
    }
    true
}

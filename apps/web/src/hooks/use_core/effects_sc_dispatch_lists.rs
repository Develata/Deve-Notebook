use deve_core::protocol::ServerMessage;
use leptos::prelude::{GetUntracked, Set};

use super::super::effects_sc_apply::apply_doc_diff;
use super::super::effects_sc_state::{
    changes_list_matches_request, commit_diff_matches_request, commit_history_matches_request,
    doc_diff_matches_request,
};
use super::ScMessageContext;

pub(crate) fn handle_sc_list_message(
    msg: &ServerMessage,
    ctx: &ScMessageContext<'_>,
    active_scope_nonce: u64,
) -> bool {
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
            ctx.set_notice.set(None);
            ctx.set_changes_request_id.set(None);
            ctx.set_staged.set(staged.clone());
            ctx.set_unstaged.set(unstaged.clone());
            true
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
            ctx.set_notice.set(None);
            ctx.set_commit_history_request_id.set(None);
            ctx.set_history.set(commits.clone());
            true
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
            ctx.set_notice.set(None);
            ctx.set_doc_diff_request_id.set(None);
            apply_doc_diff(path, old_content, new_content, ctx.set_diff);
            true
        }
        ServerMessage::MergeConflict {
            repo_id,
            branch,
            scope_nonce,
            path,
            current_content,
            incoming_content,
            actions,
            ..
        } => {
            if !ctx.in_scope(repo_id, branch) || *scope_nonce != Some(active_scope_nonce) {
                return true;
            }
            ctx.set_notice.set(None);
            leptos::logging::log!("收到合并冲突: {} ({} actions)", path, actions.len());
            apply_doc_diff(path, current_content, incoming_content, ctx.set_diff);
            true
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
            ctx.set_notice.set(None);
            ctx.set_commit_diff_request_id.set(None);
            leptos::logging::log!("收到提交差异: {} 个文件变更", diffs.len());
            ctx.set_commit_diff.set(diffs.clone());
            true
        }
        _ => false,
    }
}

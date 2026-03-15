// apps/web/src/hooks/use_core/effects_sc.rs
//! # Source Control 消息处理 (Effects - SC)
//!
//! 从 `effects.rs` 拆分而来。处理版本控制相关的 ServerMessage。

use deve_core::protocol::ServerMessage;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use leptos::prelude::*;

use super::diff_session::DiffSessionWire;
use super::effects_sc_apply::{
    FsRefreshSignals, apply_doc_diff, refresh_after_commit, refresh_after_fs_change,
};
use super::types::PendingBranchTarget;

#[allow(unused_imports)]
pub(crate) use super::effects_sc_scope::{matches_current_repo, matches_current_scope};
pub(crate) use super::effects_sc_state::{
    changes_list_matches_request, clear_repo_scoped_state, commit_diff_matches_request,
    commit_history_matches_request, doc_diff_matches_request, scoped_ack_matches,
};

#[cfg(test)]
#[path = "effects_sc_test.rs"]
mod tests;

/// 处理 Source Control 相关消息
///
/// **返回**: `true` 表示消息已处理, `false` 表示非 SC 消息。
#[allow(clippy::too_many_arguments)]
pub fn handle_sc_message(
    msg: &ServerMessage,
    set_staged: WriteSignal<Vec<ChangeEntry>>,
    set_unstaged: WriteSignal<Vec<ChangeEntry>>,
    changes_request_id: ReadSignal<Option<String>>,
    set_changes_request_id: WriteSignal<Option<String>>,
    set_history: WriteSignal<Vec<CommitInfo>>,
    commit_history_request_id: ReadSignal<Option<String>>,
    set_commit_history_request_id: WriteSignal<Option<String>>,
    set_doc_list_request_id: WriteSignal<Option<String>>,
    set_tree_request_id: WriteSignal<Option<String>>,
    doc_diff_request_id: ReadSignal<Option<String>>,
    set_doc_diff_request_id: WriteSignal<Option<String>>,
    set_diff: WriteSignal<Option<DiffSessionWire>>,
    commit_diff_request_id: ReadSignal<Option<String>>,
    set_commit_diff_request_id: WriteSignal<Option<String>>,
    set_commit_diff: WriteSignal<Vec<CommitFileDiff>>,
    current_repo_id: ReadSignal<Option<String>>,
    active_branch: ReadSignal<Option<deve_core::models::PeerId>>,
    pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pending_repo_switch: ReadSignal<Option<String>>,
    current_scope_nonce: ReadSignal<u64>,
    schedule_refresh: &dyn Fn(),
    ws: &crate::api::WsService,
) -> bool {
    let in_scope = |repo_id, branch| {
        matches_current_scope(
            repo_id,
            branch,
            current_repo_id,
            active_branch,
            pending_branch_switch,
            pending_repo_switch,
        )
    };
    match msg {
        ServerMessage::ChangesList {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            staged,
            unstaged,
        } => {
            if !in_scope(repo_id, branch) {
                return true;
            }
            if !changes_list_matches_request(
                request_id,
                changes_request_id.get_untracked(),
                *scope_nonce,
                current_scope_nonce.get_untracked(),
            ) {
                return true;
            }
            set_changes_request_id.set(None);
            set_staged.set(staged.clone());
            set_unstaged.set(unstaged.clone());
        }
        ServerMessage::CommitHistory {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            commits,
        } => {
            if !in_scope(repo_id, branch) {
                return true;
            }
            if !commit_history_matches_request(
                request_id,
                commit_history_request_id.get_untracked(),
                *scope_nonce,
                current_scope_nonce.get_untracked(),
            ) {
                return true;
            }
            set_commit_history_request_id.set(None);
            set_history.set(commits.clone());
        }
        ServerMessage::StageAck {
            repo_id,
            branch,
            scope_nonce,
            path,
        } => {
            if !in_scope(repo_id, branch)
                || !scoped_ack_matches(*scope_nonce, current_scope_nonce.get_untracked())
            {
                return true;
            }
            leptos::logging::log!("已暂存: {}", path);
            schedule_refresh();
        }
        ServerMessage::UnstageAck {
            repo_id,
            branch,
            scope_nonce,
            path,
        } => {
            if !in_scope(repo_id, branch)
                || !scoped_ack_matches(*scope_nonce, current_scope_nonce.get_untracked())
            {
                return true;
            }
            leptos::logging::log!("已取消暂存: {}", path);
            schedule_refresh();
        }
        ServerMessage::DiscardAck {
            repo_id,
            branch,
            scope_nonce,
            path,
        } => {
            if !in_scope(repo_id, branch)
                || !scoped_ack_matches(*scope_nonce, current_scope_nonce.get_untracked())
            {
                return true;
            }
            leptos::logging::log!("已放弃变更: {}", path);
            schedule_refresh();
        }
        ServerMessage::CommitAck {
            commit_id,
            repo_id,
            branch,
            scope_nonce,
            ..
        } => {
            if !in_scope(repo_id, branch)
                || !scoped_ack_matches(*scope_nonce, current_scope_nonce.get_untracked())
            {
                return true;
            }
            refresh_after_commit(
                commit_id,
                current_scope_nonce.get_untracked(),
                set_changes_request_id,
                set_commit_history_request_id,
                ws,
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
            if !in_scope(repo_id, branch) {
                return true;
            }
            if !doc_diff_matches_request(
                request_id,
                doc_diff_request_id.get_untracked(),
                *scope_nonce,
                current_scope_nonce.get_untracked(),
            ) {
                return true;
            }
            set_doc_diff_request_id.set(None);
            apply_doc_diff(path, old_content, new_content, set_diff);
        }
        ServerMessage::FsChangeDetected {
            repo_id,
            branch,
            scope_nonce,
            path,
            change_type,
            has_conflict,
        } => {
            if !in_scope(repo_id, branch)
                || !scoped_ack_matches(*scope_nonce, current_scope_nonce.get_untracked())
            {
                return true;
            }
            refresh_after_fs_change(
                path,
                change_type,
                *has_conflict,
                FsRefreshSignals {
                    current_scope_nonce: current_scope_nonce.get_untracked(),
                    set_doc_list_request_id,
                    set_tree_request_id,
                },
                schedule_refresh,
                ws,
            );
        }
        ServerMessage::CommitDiffResult {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            diffs,
        } => {
            if !in_scope(repo_id, branch) {
                return true;
            }
            if !commit_diff_matches_request(
                request_id,
                commit_diff_request_id.get_untracked(),
                *scope_nonce,
                current_scope_nonce.get_untracked(),
            ) {
                return true;
            }
            set_commit_diff_request_id.set(None);
            leptos::logging::log!("收到提交差异: {} 个文件变更", diffs.len());
            set_commit_diff.set(diffs.clone());
        }
        ServerMessage::ConflictResolved {
            repo_id,
            branch,
            scope_nonce,
            path,
            resolution,
        } => {
            if !in_scope(repo_id, branch)
                || !scoped_ack_matches(*scope_nonce, current_scope_nonce.get_untracked())
            {
                return true;
            }
            leptos::logging::log!("冲突已解决: {} ({})", path, resolution);
            schedule_refresh();
        }
        _ => return false,
    }
    true
}

// apps/web/src/hooks/use_core/effects_sc.rs
//! # Source Control 消息处理 (Effects - SC)
//!
//! 从 `effects.rs` 拆分而来。处理版本控制相关的 ServerMessage。

use deve_core::protocol::{ClientMessage, ServerMessage};
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use leptos::prelude::*;

use super::diff_session::DiffSessionWire;

/// 处理 Source Control 相关消息
///
/// **返回**: `true` 表示消息已处理, `false` 表示非 SC 消息。
#[allow(clippy::too_many_arguments)]
pub fn handle_sc_message(
    msg: &ServerMessage,
    set_staged: WriteSignal<Vec<ChangeEntry>>,
    set_unstaged: WriteSignal<Vec<ChangeEntry>>,
    set_history: WriteSignal<Vec<CommitInfo>>,
    set_diff: WriteSignal<Option<DiffSessionWire>>,
    set_commit_diff: WriteSignal<Vec<CommitFileDiff>>,
    current_repo_id: ReadSignal<Option<String>>,
    schedule_refresh: &dyn Fn(),
    ws: &crate::api::WsService,
) -> bool {
    match msg {
        ServerMessage::ChangesList { staged, unstaged } => {
            set_staged.set(staged.clone());
            set_unstaged.set(unstaged.clone());
        }
        ServerMessage::CommitHistory { commits } => {
            set_history.set(commits.clone());
        }
        ServerMessage::StageAck { path } => {
            leptos::logging::log!("已暂存: {}", path);
            schedule_refresh();
        }
        ServerMessage::UnstageAck { path } => {
            leptos::logging::log!("已取消暂存: {}", path);
            schedule_refresh();
        }
        ServerMessage::DiscardAck { path } => {
            leptos::logging::log!("已放弃变更: {}", path);
            schedule_refresh();
        }
        ServerMessage::CommitAck {
            commit_id, repo_id, ..
        } => {
            if !matches_current_repo(repo_id, current_repo_id) {
                return true;
            }
            leptos::logging::log!("已提交: {}", commit_id);
            ws.send(ClientMessage::GetChanges);
            ws.send(ClientMessage::GetCommitHistory { limit: 50 });
        }
        ServerMessage::DocDiff {
            path,
            old_content,
            new_content,
        } => {
            leptos::logging::log!("收到 Diff: {}", path);
            set_diff.set(Some(DiffSessionWire::new(
                path.clone(),
                old_content.clone(),
                new_content.clone(),
            )));
            // 计算行级差异并推送到 CodeMirror Gutter
            let ranges =
                deve_core::source_control::line_diff::compute_line_ranges(old_content, new_content);
            if let Ok(json) = serde_json::to_string(&ranges) {
                crate::editor::ffi::update_gutter_diff(&json);
            }
        }
        ServerMessage::FsChangeDetected {
            repo_id,
            path,
            change_type,
            has_conflict,
        } => {
            if !matches_current_repo(repo_id, current_repo_id) {
                return true;
            }
            let conflict_tag = if *has_conflict { " [冲突]" } else { "" };
            leptos::logging::log!("文件变更: {} ({}){}", path, change_type, conflict_tag);
            schedule_refresh();
            ws.send(ClientMessage::ListDocs);
        }
        ServerMessage::CommitDiffResult { diffs } => {
            leptos::logging::log!("收到提交差异: {} 个文件变更", diffs.len());
            set_commit_diff.set(diffs.clone());
        }
        ServerMessage::ConflictResolved { path, resolution } => {
            leptos::logging::log!("冲突已解决: {} ({})", path, resolution);
            schedule_refresh();
        }
        _ => return false,
    }
    true
}

pub(super) fn matches_current_repo(
    repo_id: &Option<uuid::Uuid>,
    current_repo_id: ReadSignal<Option<String>>,
) -> bool {
    match (repo_id, current_repo_id.get_untracked()) {
        (Some(repo_id), Some(current_repo_id)) => current_repo_id == repo_id.to_string(),
        (Some(_), None) => false,
        (None, _) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::matches_current_repo;
    use leptos::prelude::*;

    #[test]
    fn ignores_repo_scoped_messages_before_repo_scope_is_ready() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_repo_id, _) = signal(None::<String>);
        assert!(!matches_current_repo(&Some(uuid::Uuid::new_v4()), current_repo_id));
        assert!(matches_current_repo(&None, current_repo_id));
    }
}

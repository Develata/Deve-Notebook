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
        ServerMessage::ChangesList {
            repo_id,
            staged,
            unstaged,
        } => {
            if !matches_current_repo(repo_id, current_repo_id) {
                return true;
            }
            set_staged.set(staged.clone());
            set_unstaged.set(unstaged.clone());
        }
        ServerMessage::CommitHistory { repo_id, commits } => {
            if !matches_current_repo(repo_id, current_repo_id) {
                return true;
            }
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
            repo_id,
            path,
            old_content,
            new_content,
        } => {
            if !matches_current_repo(repo_id, current_repo_id) {
                return true;
            }
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
        ServerMessage::CommitDiffResult { repo_id, diffs } => {
            if !matches_current_repo(repo_id, current_repo_id) {
                return true;
            }
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

pub fn clear_repo_scoped_state(
    set_staged: WriteSignal<Vec<ChangeEntry>>,
    set_unstaged: WriteSignal<Vec<ChangeEntry>>,
    set_history: WriteSignal<Vec<CommitInfo>>,
    set_diff: WriteSignal<Option<DiffSessionWire>>,
    set_commit_diff: WriteSignal<Vec<CommitFileDiff>>,
) {
    set_staged.set(Vec::new());
    set_unstaged.set(Vec::new());
    set_history.set(Vec::new());
    set_diff.set(None);
    set_commit_diff.set(Vec::new());
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
    use super::{clear_repo_scoped_state, matches_current_repo};
    use crate::hooks::use_core::diff_session::DiffSessionWire;
    use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
    use leptos::prelude::*;

    #[test]
    fn ignores_repo_scoped_messages_before_repo_scope_is_ready() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (current_repo_id, _) = signal(None::<String>);
        assert!(!matches_current_repo(
            &Some(uuid::Uuid::new_v4()),
            current_repo_id
        ));
        assert!(matches_current_repo(&None, current_repo_id));
    }

    #[test]
    fn clear_repo_scoped_state_resets_source_control_view() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let entry = ChangeEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
            status: deve_core::source_control::ChangeStatus::Modified,
            has_conflict: false,
        };
        let (staged, set_staged) = signal(vec![entry.clone()]);
        let (unstaged, set_unstaged) = signal(vec![entry]);
        let (history, set_history) = signal(vec![CommitInfo {
            id: "c1".into(),
            parent_id: None,
            message: "msg".into(),
            timestamp: 1,
            doc_count: 1,
            ledger_seq: 1,
        }]);
        let (diff, set_diff) = signal(Some(DiffSessionWire {
            path: "a.md".into(),
            old_content: "old".into(),
            new_content: "new".into(),
            opened_at_ms: 1,
        }));
        let (commit_diff, set_commit_diff) = signal(vec![CommitFileDiff {
            path: "notes/a.md".into(),
            status: deve_core::source_control::ChangeStatus::Modified,
            previous_path: None,
            old_content: "old".into(),
            new_content: "new".into(),
        }]);

        clear_repo_scoped_state(
            set_staged,
            set_unstaged,
            set_history,
            set_diff,
            set_commit_diff,
        );

        assert!(staged.get_untracked().is_empty());
        assert!(unstaged.get_untracked().is_empty());
        assert!(history.get_untracked().is_empty());
        assert_eq!(diff.get_untracked(), None);
        assert!(commit_diff.get_untracked().is_empty());
    }
}

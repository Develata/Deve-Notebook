// apps\web\src\i18n
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! # I18n Source Control Module (源代码管理翻译)
//!
//! 包含版本控制面板相关的翻译字符串。

#![allow(dead_code)] // 翻译字符串按需使用

use super::Locale;

mod actions;
mod gate;

pub use super::source_control_git::*;
pub use super::source_control_graph::*;
pub use super::source_control_history::*;
pub use super::source_control_native::*;
pub use actions::*;
pub use gate::*;

pub fn title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Source Control",
        Locale::Zh => "源代码管理",
    }
}

pub fn repositories(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Repositories",
        Locale::Zh => "存储库",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_repo_selected_is_localized() {
        assert_eq!(no_repo_selected(Locale::En), "No repo selected");
        assert_eq!(no_repo_selected(Locale::Zh), "尚未选择仓库");
    }

    #[test]
    fn source_control_blocking_hints_are_localized() {
        assert_eq!(scope_switching(Locale::Zh), "切换作用域中...");
        assert_eq!(no_changes(Locale::En), "No changes");
        assert_eq!(no_changes(Locale::Zh), "没有更改");
        assert_eq!(
            session_expired_hint(Locale::En),
            "Sign in again before staging, discarding, or committing changes."
        );
        assert_eq!(
            handshaking_repo_hint(Locale::Zh),
            "当前仓库仍在协商写入权限，请稍后再试。"
        );
    }

    #[test]
    fn source_control_notice_copy_is_localized() {
        assert_eq!(diff_unavailable(Locale::Zh), "无法显示差异");
        assert_eq!(
            deleted_change_no_doc_diff(Locale::En, "old.md"),
            "No diff is available for deleted change old.md because it has no document identity."
        );
        assert_eq!(
            git_import_cli_only_title(Locale::En),
            "Git import is CLI-only"
        );
        assert!(git_import_cli_only_hint(Locale::En).contains("deve_cli git import --apply"));
        assert_eq!(
            establish_branch_unavailable_title(Locale::En),
            "P2P branch creation is unavailable"
        );
        assert!(establish_branch_unavailable_hint(Locale::Zh).contains("P2P: Merge Peer"));
        assert_eq!(
            git_push_cli_only_title(Locale::Zh),
            "Git mirror 推送只能通过 CLI 执行"
        );
        assert!(git_push_cli_only_hint(Locale::Zh).contains("deve_cli git push"));
        assert_eq!(refresh_change_list(Locale::Zh), "请刷新更改列表后再试。");
    }

    #[test]
    fn source_control_diff_copy_is_localized() {
        assert_eq!(open_diff(Locale::En), "Open Diff");
        assert_eq!(open_diff(Locale::Zh), "打开差异");
        assert_eq!(
            confirmed_ledger_hint(Locale::Zh),
            "账本已确认更改不支持逐文件暂存或放弃；提交会整体覆盖本组。"
        );
        assert_eq!(loading_commit_diff(Locale::Zh), "正在加载提交差异...");
        assert_eq!(counterpart_staged_badge(Locale::Zh), "暂存区");
    }
}

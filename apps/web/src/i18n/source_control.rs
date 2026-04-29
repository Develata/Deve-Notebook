// apps\web\src\i18n
//! plan_ref:
//!   - 11_i18n#i18n-keys-reference
//!
//! # I18n Source Control Module (源代码管理翻译)
//!
//! 包含版本控制面板相关的翻译字符串。

#![allow(dead_code)] // 翻译字符串按需使用

use super::Locale;

pub use super::source_control_git::*;
pub use super::source_control_native::*;

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

pub fn no_repo_selected(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No repo selected",
        Locale::Zh => "尚未选择仓库",
    }
}

pub fn scope_switching(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Switching scope...",
        Locale::Zh => "切换作用域中...",
    }
}

pub fn session_expired_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sign in again before staging, discarding, or committing changes.",
        Locale::Zh => "请重新登录后再暂存、放弃或提交更改。",
    }
}

pub fn offline_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Wait for the connection to recover before changing Source Control state.",
        Locale::Zh => "请等待连接恢复后再修改源代码管理状态。",
    }
}

pub fn reconnecting_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "The client is reconnecting. Source Control actions will resume automatically."
        }
        Locale::Zh => "客户端正在重连，源代码管理操作会在恢复后自动可用。",
    }
}

pub fn snapshot_loading_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Wait for the current repo snapshot to finish loading.",
        Locale::Zh => "请等待当前仓库快照加载完成。",
    }
}

pub fn scope_switching_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Wait for the repo or branch switch to finish before editing changes.",
        Locale::Zh => "请等待仓库或分支切换完成后再修改更改列表。",
    }
}

pub fn no_repo_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Select an active repo before using Source Control actions.",
        Locale::Zh => "请先选择激活仓库，再使用源代码管理操作。",
    }
}

pub fn handshaking_repo_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "This repo is still negotiating writer access. Try again in a moment.",
        Locale::Zh => "当前仓库仍在协商写入权限，请稍后再试。",
    }
}

pub fn diff_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Diff unavailable",
        Locale::Zh => "无法显示差异",
    }
}

pub fn deleted_change_no_doc_diff(locale: Locale, path: &str) -> String {
    match locale {
        Locale::En => format!(
            "No diff is available for deleted change {path} because it has no document identity."
        ),
        Locale::Zh => format!("删除变更 {path} 没有文档身份，因此当前无法生成可显示的差异。"),
    }
}

pub fn legacy_commit_unprojectable(locale: Locale, commit: Option<&str>) -> String {
    match (locale, commit) {
        (Locale::En, Some(commit)) => format!(
            "Commit {commit} contains legacy content without structure projection, so Deve-Note cannot reconstruct a path-safe diff."
        ),
        (Locale::Zh, Some(commit)) => format!(
            "提交 {commit} 包含缺少结构投影的旧内容，Deve-Note 无法安全重建带路径语义的差异。"
        ),
        (Locale::En, None) => {
            "This legacy commit contains content without structure projection, so Deve-Note cannot reconstruct a path-safe diff.".to_string()
        }
        (Locale::Zh, None) => {
            "该旧提交包含缺少结构投影的内容，Deve-Note 无法安全重建带路径语义的差异。".to_string()
        }
    }
}

pub fn stage_files_before_commit(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Stage files before trying to commit.",
        Locale::Zh => "请先暂存文件，再执行提交。",
    }
}

pub fn refresh_change_list(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Refresh the change list and try again.",
        Locale::Zh => "请刷新更改列表后再试。",
    }
}

pub fn selected_item_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The selected Source Control item is no longer available.",
        Locale::Zh => "当前选中的源代码管理条目已不存在。",
    }
}

pub fn loading_commit_diff(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Loading commit diff...",
        Locale::Zh => "正在加载提交差异...",
    }
}

pub fn counterpart_staged_badge(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "IDX",
        Locale::Zh => "暂存区",
    }
}

pub fn counterpart_working_tree_badge(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "WT",
        Locale::Zh => "工作区",
    }
}

pub fn counterpart_staged_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Also present in Staged Changes",
        Locale::Zh => "对应改动也存在于暂存区",
    }
}

pub fn counterpart_working_tree_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Also modified in Working Directory",
        Locale::Zh => "对应改动也存在于工作区",
    }
}

pub fn history_compare_message(locale: Locale, base_label: &str, target_label: &str) -> String {
    match locale {
        Locale::En => format!("Comparing {base_label} -> {target_label}."),
        Locale::Zh => format!("正在比较 {base_label} -> {target_label}。"),
    }
}

pub fn history_base_selected_message(locale: Locale, base_label: &str) -> String {
    match locale {
        Locale::En => format!("Base {base_label} selected. Click another commit to compare."),
        Locale::Zh => format!("已选择基准提交 {base_label}。点击另一条提交即可比较。"),
    }
}

pub fn history_selected_target_message(locale: Locale, target_label: &str) -> String {
    match locale {
        Locale::En => format!("Selected {target_label}. Use it as the comparison base?"),
        Locale::Zh => format!("已选择提交 {target_label}。要把它设为比较基准吗？"),
    }
}

pub fn clear_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Clear",
        Locale::Zh => "清除",
    }
}

pub fn use_as_base_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Use as Base",
        Locale::Zh => "设为基准",
    }
}

pub fn loading_history(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Loading history...",
        Locale::Zh => "正在加载历史记录...",
    }
}

pub fn no_commit_history(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No commit history yet on this branch.",
        Locale::Zh => "这个分支上还没有提交历史。",
    }
}

pub fn no_diff_between_commits(locale: Locale, base: &str, target: &str) -> String {
    match locale {
        Locale::En => format!("No file-level diff available between {base} and {target}."),
        Locale::Zh => format!("提交 {base} 与 {target} 之间没有可展示的文件级差异。"),
    }
}

pub fn no_diff_for_commit(locale: Locale) -> String {
    match locale {
        Locale::En => "No file-level diff available for this commit.".to_string(),
        Locale::Zh => "这个提交没有可展示的文件级差异。".to_string(),
    }
}

pub fn changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Changes",
        Locale::Zh => "更改",
    }
}

pub fn staged_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Staged Changes",
        Locale::Zh => "暂存的更改",
    }
}

pub fn history(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "History",
        Locale::Zh => "历史记录",
    }
}

pub fn graph(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Graph",
        Locale::Zh => "图形",
    }
}

pub fn open_file(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open File",
        Locale::Zh => "打开文件",
    }
}

pub fn stage_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Stage Changes",
        Locale::Zh => "暂存更改",
    }
}

pub fn unstage_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unstage Changes",
        Locale::Zh => "取消暂存更改",
    }
}

pub fn discard_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Discard Changes",
        Locale::Zh => "放弃更改",
    }
}

pub fn stage_all_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Stage All Changes",
        Locale::Zh => "暂存全部更改",
    }
}

pub fn unstage_all_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unstage All Changes",
        Locale::Zh => "取消暂存全部更改",
    }
}

pub fn discard_all_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Discard All Changes",
        Locale::Zh => "放弃全部更改",
    }
}

pub fn commit_message_placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Message (Ctrl+Enter to commit on the current branch)",
        Locale::Zh => "提交信息（Ctrl+Enter 在当前分支提交）",
    }
}

pub fn commit(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Commit",
        Locale::Zh => "提交",
    }
}

pub fn generate_commit_message(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Generate Commit Message",
        Locale::Zh => "生成提交信息",
    }
}

pub fn generate(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Generate",
        Locale::Zh => "生成",
    }
}

pub fn commit_and_push(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Commit & Push",
        Locale::Zh => "提交并推送",
    }
}

pub fn generating(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Generating...",
        Locale::Zh => "生成中...",
    }
}

pub fn branch_main(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Local",
        Locale::Zh => "本地",
    }
}

pub fn keep_file_system(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Keep File System",
        Locale::Zh => "保留文件系统版本",
    }
}

pub fn keep_ledger(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Keep Ledger",
        Locale::Zh => "保留账本版本",
    }
}

pub fn generate_prompt(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Generate a concise git commit message for these staged changes:",
        Locale::Zh => "为以下暂存的更改生成简洁的 Git 提交信息：",
    }
}

pub fn remote_branch_readonly(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote branches are read-only in Source Control.",
        Locale::Zh => "远端分支在源代码管理中为只读。",
    }
}

pub fn remote_branch_readonly_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Switch back to Local to view changes, stage files, or commit.",
        Locale::Zh => "切回本地分支后才能查看变更、暂存文件或提交。",
    }
}

pub fn history_base_badge(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Base",
        Locale::Zh => "基准",
    }
}

pub fn history_target_badge(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Target",
        Locale::Zh => "目标",
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
            git_push_cli_only_title(Locale::Zh),
            "Git mirror 推送只能通过 CLI 执行"
        );
        assert!(git_push_cli_only_hint(Locale::Zh).contains("deve_cli git push"));
        assert_eq!(refresh_change_list(Locale::Zh), "请刷新更改列表后再试。");
    }

    #[test]
    fn source_control_history_copy_is_localized() {
        assert_eq!(loading_commit_diff(Locale::Zh), "正在加载提交差异...");
        assert_eq!(counterpart_staged_badge(Locale::Zh), "暂存区");
        assert_eq!(
            history_compare_message(Locale::En, "abc1234", "def5678"),
            "Comparing abc1234 -> def5678."
        );
        assert_eq!(
            no_diff_between_commits(Locale::En, "abc1234", "def5678"),
            "No file-level diff available between abc1234 and def5678."
        );
    }
}

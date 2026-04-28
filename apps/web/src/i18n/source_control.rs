// apps\web\src\i18n
//! # I18n Source Control Module (源代码管理翻译)
//!
//! 包含版本控制面板相关的翻译字符串。

#![allow(dead_code)] // 翻译字符串按需使用

use super::Locale;

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
}

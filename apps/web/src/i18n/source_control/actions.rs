//! Source Control action label copy.
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference

use super::Locale;

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

pub fn confirmed_ledger_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Confirmed Ledger Changes",
        Locale::Zh => "已确认账本更改",
    }
}

pub fn confirmed_ledger_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Ledger-confirmed changes cannot be staged or discarded per file; commit covers this group as one anchor."
        }
        Locale::Zh => "账本已确认更改不支持逐文件暂存或放弃；提交会整体覆盖本组。",
    }
}

pub fn no_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No changes",
        Locale::Zh => "没有更改",
    }
}

pub fn graph(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Graph",
        Locale::Zh => "图形",
    }
}

pub fn open_diff(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open Diff",
        Locale::Zh => "打开差异",
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

pub fn commit_disabled_no_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No staged or confirmed ledger changes to commit",
        Locale::Zh => "没有可提交的暂存或已确认账本更改",
    }
}

pub fn commit_disabled_no_changes_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Stage changes or edit the ledger before committing.",
        Locale::Zh => "请先暂存更改，或在账本出现已确认更改后再提交。",
    }
}

pub fn commit_disabled_empty_message(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Enter a commit message before committing",
        Locale::Zh => "请输入提交信息后再提交",
    }
}

pub fn commit(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Commit",
        Locale::Zh => "提交",
    }
}

pub fn commit_message_required(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Enter a commit message before committing.",
        Locale::Zh => "请输入提交信息后再提交。",
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
        Locale::En => "Generate a concise git commit message for these changes:",
        Locale::Zh => "为以下更改生成简洁的 Git 提交信息：",
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

pub fn readonly_write_gate_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Restore a writable local state before viewing changes, staging files, or committing."
        }
        Locale::Zh => "恢复本地可写状态后才能查看变更、暂存文件或提交。",
    }
}

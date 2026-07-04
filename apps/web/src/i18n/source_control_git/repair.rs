//! ngit mirror repair copy.
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#cli-commands

use crate::i18n::Locale;

pub fn git_repair_cli_only_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit mirror repair is CLI-only",
        Locale::Zh => "ngit mirror 修复只能通过 CLI 执行",
    }
}

pub fn git_repair_cli_only_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Run `deve_cli ngit status --repo <repo>` to inspect `repair_action[...]`, fix the subject, then run `deve_cli ngit export --repo <repo> --retry-out-of-sync`."
        }
        Locale::Zh => {
            "请运行 `deve_cli ngit status --repo <repo>` 查看 `repair_action[...]`，修复 subject 后再运行 `deve_cli ngit export --repo <repo> --retry-out-of-sync`。"
        }
    }
}

pub fn git_repair_cli_only_details(locale: Locale) -> [&'static str; 4] {
    match locale {
        Locale::En => [
            "`repair_action[...]` is diagnostic only; Web never runs ngit repair automatically.",
            "Projection/path blockers must be fixed in Deve or the workspace before retry.",
            "Dirty Git main worktree blockers must be cleaned or imported with `deve_cli ngit import --apply --repo <repo>`.",
            "Retry export only after blockers are fixed: `deve_cli ngit export --repo <repo> --retry-out-of-sync`.",
        ],
        Locale::Zh => [
            "`repair_action[...]` 只用于诊断；Web 不会自动执行 Git 修复。",
            "projection/path blocker 必须先在 Deve 或 workspace 中修复，再重试。",
            "Git 工作区脏 blocker 需要先清理，或用 `deve_cli ngit import --apply --repo <repo>` 导入。",
            "blocker 修复后再重试导出：`deve_cli ngit export --repo <repo> --retry-out-of-sync`。",
        ],
    }
}

pub fn git_repair_review_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Read-only repair review",
        Locale::Zh => "只读修复审阅",
    }
}

pub fn git_repair_review_loading(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Loading record-level repair data...",
        Locale::Zh => "正在加载 record-level 修复数据...",
    }
}

pub fn git_repair_review_load_failed(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Record-level repair data could not load; showing CLI fallback.",
        Locale::Zh => "无法加载 record-level 修复数据；显示 CLI fallback。",
    }
}

pub fn git_repair_review_empty(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No out-of-sync records are currently reported; showing CLI fallback.",
        Locale::Zh => "当前没有 out-of-sync 记录；显示 CLI fallback。",
    }
}

pub fn git_repair_review_loaded_count(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!("{count} out-of-sync record(s) from server-side review data."),
        Locale::Zh => format!("来自 server-side review data 的 {count} 条 out-of-sync 记录。"),
    }
}

pub fn git_repair_review_fallback_record(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "CLI fallback",
        Locale::Zh => "CLI fallback",
    }
}

pub fn git_repair_action_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Repair action",
        Locale::Zh => "修复动作",
    }
}

pub fn git_repair_action_value(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "`repair_action[...]` from CLI status",
        Locale::Zh => "CLI status 中的 `repair_action[...]`",
    }
}

pub fn git_repair_guidance_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Guidance",
        Locale::Zh => "指引",
    }
}

pub fn git_repair_commit_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Commit",
        Locale::Zh => "Commit",
    }
}

pub fn git_repair_guidance_value(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "`repair_guidance[...]`: manual_only=yes",
        Locale::Zh => "`repair_guidance[...]`: manual_only=yes",
    }
}

pub fn git_repair_subject_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Subject",
        Locale::Zh => "Subject",
    }
}

pub fn git_repair_subject_value(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Fix the subject reported by `repair_action[...]` before retry.",
        Locale::Zh => "重试前先修复 `repair_action[...]` 报告的 subject。",
    }
}

pub fn git_repair_next_step_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Next step",
        Locale::Zh => "下一步",
    }
}

pub fn git_repair_next_step_value(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Use CLI status to inspect blockers, repair them manually, then retry export."
        }
        Locale::Zh => "用 CLI status 检查 blocker，手动修复后再重试 export。",
    }
}

pub fn git_repair_retry_command_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Retry command",
        Locale::Zh => "重试命令",
    }
}

pub fn git_repair_retry_command() -> &'static str {
    "deve_cli ngit export --repo <repo> --retry-out-of-sync"
}

pub fn git_repair_authority_note(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "This panel is read-only: Web does not run ngit repair, and `.notegit` remains the authority."
        }
        Locale::Zh => "此面板只读：Web 不执行 ngit repair，`.notegit` 仍是 authority。",
    }
}

//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts
//!

use super::Locale;

mod repair;

pub use repair::*;

pub fn git_bridge_mode_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "NoteGit authority; Git is an optional bridge",
        Locale::Zh => "NoteGit 是 authority；Git 是可选 bridge",
    }
}

pub fn git_bridge_mode_badge(_locale: Locale, mode: &str) -> String {
    match mode {
        "mirror" => "NoteGit + Git mirror".to_string(),
        "off" => "NoteGit only".to_string(),
        "unknown" => "NoteGit + Git unknown".to_string(),
        _ => "NoteGit + Git unknown".to_string(),
    }
}

pub fn git_status_cli_only_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git status is CLI-only",
        Locale::Zh => "Git status 只能通过 CLI 查看",
    }
}

pub fn git_status_cli_only_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Run `deve_cli git status --repo <repo>` to inspect mirror readiness and queue state."
        }
        Locale::Zh => {
            "请运行 `deve_cli git status --repo <repo>` 查看 mirror readiness 与队列状态。"
        }
    }
}

pub fn git_mirror_cli_only_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git mirror execution is CLI-only",
        Locale::Zh => "Git mirror 执行只能通过 CLI 完成",
    }
}

pub fn git_mirror_cli_only_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Run `deve_cli git mirror --repo <repo>` to execute queued mirror commits after preflight."
        }
        Locale::Zh => {
            "请运行 `deve_cli git mirror --repo <repo>`，在 preflight 后执行 queued mirror commits。"
        }
    }
}

pub fn git_export_cli_only_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git mirror export is CLI-only",
        Locale::Zh => "Git mirror 导出只能通过 CLI 执行",
    }
}

pub fn git_export_cli_only_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Run `deve_cli git export --repo <repo>` to create Git mirror commits from Deve projections."
        }
        Locale::Zh => {
            "请运行 `deve_cli git export --repo <repo>`，从 Deve projection 建立 Git mirror commits。"
        }
    }
}

pub fn git_import_cli_only_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git import is CLI-only",
        Locale::Zh => "Git import 只能通过 CLI 执行",
    }
}

pub fn git_import_cli_only_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Run `deve_cli git import --apply --repo <repo>` first. Blockers must be fixed in Git; imported conflicts then use Keep File System / Keep Ledger here."
        }
        Locale::Zh => {
            "请先运行 `deve_cli git import --apply --repo <repo>`。blocker 需要在 Git 侧修复；导入后的冲突再在这里选择保留文件系统或账本版本。"
        }
    }
}

pub fn git_push_cli_only_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git mirror push is CLI-only",
        Locale::Zh => "Git mirror 推送只能通过 CLI 执行",
    }
}

pub fn git_push_cli_only_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Run `deve_cli git push --repo <repo>`. Add `--remote <remote> --branch <branch>` when upstream/origin is not configured."
        }
        Locale::Zh => {
            "请运行 `deve_cli git push --repo <repo>`。如果未配置 upstream/origin，请加 `--remote <remote> --branch <branch>`。"
        }
    }
}

pub fn git_push_cli_only_details(locale: Locale) -> [&'static str; 5] {
    match locale {
        Locale::En => [
            "Remote target: uses branch upstream first, then `origin`; detached HEAD needs `--branch`.",
            "Mirror mapping: run `deve_cli git export --repo <repo>` before push when HEAD is unmapped or queued.",
            "Out-of-sync mirror: repair, then rerun export with `--retry-out-of-sync`.",
            "Dirty Git worktree: clean Git changes or import them with `deve_cli git import --apply --repo <repo>`.",
            "Dirty Deve Source Control: stage/commit/discard pending Deve changes before pushing.",
        ],
        Locale::Zh => [
            "远端目标：优先使用当前分支 upstream，其次 fallback 到 `origin`；detached HEAD 需要显式 `--branch`。",
            "Mirror 映射：HEAD 未映射或仍有 queued 记录时，先运行 `deve_cli git export --repo <repo>`。",
            "Mirror 失配：先修复，再用 `deve_cli git export --retry-out-of-sync --repo <repo>` 重试。",
            "Git 工作区脏：先清理 Git 变更，或用 `deve_cli git import --apply --repo <repo>` 导入。",
            "Deve Source Control 脏：先暂存/提交/放弃 Deve pending 变更，再推送。",
        ],
    }
}

pub fn git_import_conflict_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Imported conflict: choose Keep File System or Keep Ledger before staging.",
        Locale::Zh => "导入冲突：暂存前请选择保留文件系统版本或账本版本。",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_bridge_source_control_copy_is_localized() {
        assert_eq!(
            git_bridge_mode_title(Locale::En),
            "NoteGit authority; Git is an optional bridge"
        );
        assert_eq!(
            git_bridge_mode_badge(Locale::Zh, "mirror"),
            "NoteGit + Git mirror"
        );
        assert_eq!(
            git_status_cli_only_title(Locale::En),
            "Git status is CLI-only"
        );
        assert!(git_status_cli_only_hint(Locale::Zh).contains("deve_cli git status"));
        assert_eq!(
            git_mirror_cli_only_title(Locale::Zh),
            "Git mirror 执行只能通过 CLI 完成"
        );
        assert!(git_mirror_cli_only_hint(Locale::En).contains("deve_cli git mirror"));
        assert_eq!(
            git_export_cli_only_title(Locale::En),
            "Git mirror export is CLI-only"
        );
        assert!(git_export_cli_only_hint(Locale::Zh).contains("deve_cli git export"));
        assert_eq!(
            git_import_cli_only_title(Locale::En),
            "Git import is CLI-only"
        );
        assert!(git_import_cli_only_hint(Locale::En).contains("deve_cli git import --apply"));
        assert_eq!(
            git_push_cli_only_title(Locale::Zh),
            "Git mirror 推送只能通过 CLI 执行"
        );
        assert!(git_push_cli_only_hint(Locale::Zh).contains("--remote <remote> --branch <branch>"));
        assert!(
            git_push_cli_only_details(Locale::En)
                .iter()
                .any(|line| line.contains("deve_cli git export --repo <repo>"))
        );
        assert!(
            git_push_cli_only_details(Locale::Zh)
                .iter()
                .any(|line| line.contains("deve_cli git import --apply --repo <repo>"))
        );
        assert_eq!(
            git_repair_cli_only_title(Locale::En),
            "Git mirror repair is CLI-only"
        );
        assert!(git_repair_cli_only_hint(Locale::Zh).contains("repair_action[...]"));
        assert!(
            git_repair_cli_only_details(Locale::En)
                .iter()
                .any(|line| line.contains("retry-out-of-sync"))
        );
        assert_eq!(git_repair_review_title(Locale::Zh), "只读修复审阅");
        assert!(git_repair_review_loading(Locale::En).contains("Loading"));
        assert!(git_repair_review_load_failed(Locale::En).contains("could not load"));
        assert!(git_repair_review_empty(Locale::En).contains("No out-of-sync"));
        assert!(git_repair_review_loaded_count(Locale::En, 2).contains("2"));
        assert_eq!(
            git_repair_review_fallback_record(Locale::Zh),
            "CLI fallback"
        );
        assert!(git_repair_guidance_value(Locale::En).contains("manual_only=yes"));
        assert_eq!(git_repair_commit_label(Locale::En), "Commit");
        assert_eq!(
            git_repair_retry_command(),
            "deve_cli git export --repo <repo> --retry-out-of-sync"
        );
        assert!(git_repair_authority_note(Locale::En).contains("read-only"));
        assert_eq!(
            git_import_conflict_title(Locale::Zh),
            "导入冲突：暂存前请选择保留文件系统版本或账本版本。"
        );
    }

    #[test]
    fn source_control_header_git_bridge_mode_badge() {
        assert_eq!(git_bridge_mode_badge(Locale::En, "off"), "NoteGit only");
        assert_eq!(
            git_bridge_mode_badge(Locale::Zh, "mirror"),
            "NoteGit + Git mirror"
        );
        assert_eq!(
            git_bridge_mode_badge(Locale::En, "native"),
            "NoteGit + Git unknown"
        );
    }
}

//! plan_ref:
//!   - 11_i18n#i18n-keys-reference
//!   - 12_commands#command-palette-shortcuts
//!

use super::Locale;

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

pub fn git_repair_cli_only_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git mirror repair is CLI-only",
        Locale::Zh => "Git mirror 修复只能通过 CLI 执行",
    }
}

pub fn git_repair_cli_only_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Run `deve_cli git status --repo <repo>` to inspect `repair_action[...]`, fix the subject, then run `deve_cli git export --repo <repo> --retry-out-of-sync`."
        }
        Locale::Zh => {
            "请运行 `deve_cli git status --repo <repo>` 查看 `repair_action[...]`，修复 subject 后再运行 `deve_cli git export --repo <repo> --retry-out-of-sync`。"
        }
    }
}

pub fn git_repair_cli_only_details(locale: Locale) -> [&'static str; 4] {
    match locale {
        Locale::En => [
            "`repair_action[...]` is diagnostic only; Web never runs Git repair automatically.",
            "Projection/path blockers must be fixed in Deve or the workspace before retry.",
            "Dirty Git worktree blockers must be cleaned or imported with `deve_cli git import --apply --repo <repo>`.",
            "Retry export only after blockers are fixed: `deve_cli git export --repo <repo> --retry-out-of-sync`.",
        ],
        Locale::Zh => [
            "`repair_action[...]` 只用于诊断；Web 不会自动执行 Git 修复。",
            "projection/path blocker 必须先在 Deve 或 workspace 中修复，再重试。",
            "Git 工作区脏 blocker 需要先清理，或用 `deve_cli git import --apply --repo <repo>` 导入。",
            "blocker 修复后再重试导出：`deve_cli git export --repo <repo> --retry-out-of-sync`。",
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
        assert_eq!(
            git_import_conflict_title(Locale::Zh),
            "导入冲突：暂存前请选择保留文件系统版本或账本版本。"
        );
    }
}

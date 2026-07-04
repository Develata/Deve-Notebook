//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn git_status_cli_only_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit status is CLI-only",
        Locale::Zh => "ngit status 只能通过 CLI 查看",
    }
}

pub fn git_status_cli_only_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Run `deve_cli ngit status --repo <repo>` to inspect mirror readiness and queue state."
        }
        Locale::Zh => {
            "请运行 `deve_cli ngit status --repo <repo>` 查看 mirror readiness 与队列状态。"
        }
    }
}

pub fn git_mirror_cli_only_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit mirror execution is CLI-only",
        Locale::Zh => "ngit mirror 执行只能通过 CLI 完成",
    }
}

pub fn git_mirror_cli_only_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Run `deve_cli ngit mirror --repo <repo>` to execute queued mirror commits after preflight."
        }
        Locale::Zh => {
            "请运行 `deve_cli ngit mirror --repo <repo>`，在 preflight 后执行 queued mirror commits。"
        }
    }
}

pub fn git_export_cli_only_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit mirror export is CLI-only",
        Locale::Zh => "ngit mirror 导出只能通过 CLI 执行",
    }
}

pub fn git_export_cli_only_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Run `deve_cli ngit export --repo <repo>` to create ngit mirror commits from Deve projections."
        }
        Locale::Zh => {
            "请运行 `deve_cli ngit export --repo <repo>`，从 Deve projection 建立 ngit mirror commits。"
        }
    }
}

pub fn git_import_cli_only_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit import is CLI-only",
        Locale::Zh => "ngit import 只能通过 CLI 执行",
    }
}

pub fn git_import_cli_only_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Run `deve_cli ngit import --apply --repo <repo>` first. Blockers must be fixed in Git; imported conflicts then use Keep File System / Keep Ledger here."
        }
        Locale::Zh => {
            "请先运行 `deve_cli ngit import --apply --repo <repo>`。blocker 需要在 Git 侧修复；导入后的冲突再在这里选择保留文件系统或账本版本。"
        }
    }
}

pub fn git_push_cli_only_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit mirror push is CLI-only",
        Locale::Zh => "ngit mirror 推送只能通过 CLI 执行",
    }
}

pub fn git_push_cli_only_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Run `deve_cli ngit push --repo <repo>`. Add `--remote <remote> --branch <branch>` when upstream/origin is not configured."
        }
        Locale::Zh => {
            "请运行 `deve_cli ngit push --repo <repo>`。如果未配置 upstream/origin，请加 `--remote <remote> --branch <branch>`。"
        }
    }
}

pub fn commit_and_push_cli_only_banner(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Commit & Push is CLI-only; create a commit first, then run `deve_cli ngit push`."
        }
        Locale::Zh => {
            "Commit & Push 只能通过 CLI 完成；请先创建提交，再运行 `deve_cli ngit push`。"
        }
    }
}

pub fn git_push_cli_only_details(locale: Locale) -> [&'static str; 5] {
    match locale {
        Locale::En => [
            "Remote target: uses branch upstream first, then `origin`; detached HEAD needs `--branch`.",
            "Mirror mapping: run `deve_cli ngit export --repo <repo>` before push when HEAD is unmapped or queued.",
            "Out-of-sync mirror: repair, then rerun export with `--retry-out-of-sync`.",
            "Dirty Git main worktree: clean Git changes or import them with `deve_cli ngit import --apply --repo <repo>`.",
            "Dirty Deve Source Control: stage/commit/discard pending Deve changes before pushing.",
        ],
        Locale::Zh => [
            "远端目标：优先使用当前分支 upstream，其次 fallback 到 `origin`；detached HEAD 需要显式 `--branch`。",
            "Mirror 映射：HEAD 未映射或仍有 queued 记录时，先运行 `deve_cli ngit export --repo <repo>`。",
            "Mirror 失配：先修复，再用 `deve_cli ngit export --retry-out-of-sync --repo <repo>` 重试。",
            "Git 工作区脏：先清理 Git 变更，或用 `deve_cli ngit import --apply --repo <repo>` 导入。",
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

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
            "Run `deve_cli git push --repo <repo>`. Export or repair the mirror first if HEAD is unmapped, queued, or out of sync."
        }
        Locale::Zh => {
            "请运行 `deve_cli git push --repo <repo>`。若 HEAD 未映射、队列未导出或 mirror 失配，请先 export/repair。"
        }
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
        assert!(git_push_cli_only_hint(Locale::Zh).contains("deve_cli git push"));
        assert_eq!(
            git_import_conflict_title(Locale::Zh),
            "导入冲突：暂存前请选择保留文件系统版本或账本版本。"
        );
    }
}

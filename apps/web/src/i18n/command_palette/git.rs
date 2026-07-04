//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn git_status(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git: Status",
        Locale::Zh => "Git: 状态",
    }
}

pub fn git_mirror(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git: Mirror",
        Locale::Zh => "Git: 执行 Mirror",
    }
}

pub fn git_export_mirror(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git: Export Mirror",
        Locale::Zh => "Git: 导出 Mirror",
    }
}

pub fn git_cli_only_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "CLI-only: Web does not execute Git writer commands",
        Locale::Zh => "CLI-only：Web 不执行 Git 写命令",
    }
}

pub fn git_import_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git: Import Changes",
        Locale::Zh => "Git: 导入外部变更",
    }
}

pub fn git_push_mirror(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git: Push Mirror",
        Locale::Zh => "Git: 推送 Mirror",
    }
}

pub fn git_repair_mirror(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Git: Repair Mirror",
        Locale::Zh => "Git: 修复 Mirror",
    }
}

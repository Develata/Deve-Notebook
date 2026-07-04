//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn ngit_status(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit: status",
        Locale::Zh => "ngit: 状态",
    }
}

pub fn ngit_mirror(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit: mirror",
        Locale::Zh => "ngit: 执行 mirror",
    }
}

pub fn ngit_export_mirror(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit: export mirror",
        Locale::Zh => "ngit: 导出 mirror",
    }
}

pub fn ngit_cli_only_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "CLI-only: Web sends no Git writer authority",
        Locale::Zh => "CLI-only：Web 不持有 Git 写 authority",
    }
}

pub fn ngit_import_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit: import changes",
        Locale::Zh => "ngit: 导入外部变更",
    }
}

pub fn ngit_push_mirror(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit: push mirror",
        Locale::Zh => "ngit: 推送 mirror",
    }
}

pub fn ngit_repair_mirror(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "ngit: repair mirror",
        Locale::Zh => "ngit: 修复 mirror",
    }
}

//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 15_settings#configuration-settings

use crate::i18n::Locale;

pub fn sync_mode(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sync Mode",
        Locale::Zh => "同步模式",
    }
}

pub fn sync_mode_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Auto: instant sync. Manual: review before merge.",
        Locale::Zh => "自动: 实时同步。手动: 合并前审查。",
    }
}

pub fn auto_mode(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Auto",
        Locale::Zh => "自动",
    }
}

pub fn manual_mode(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Manual",
        Locale::Zh => "手动",
    }
}

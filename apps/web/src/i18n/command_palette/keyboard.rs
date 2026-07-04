//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn keyboard_navigate_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "to navigate",
        Locale::Zh => "导航",
    }
}

pub fn keyboard_select_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "to select",
        Locale::Zh => "选择",
    }
}

pub fn keyboard_close_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "to close",
        Locale::Zh => "关闭",
    }
}

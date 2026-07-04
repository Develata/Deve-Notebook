//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! Bottom bar field labels and action labels.

use super::super::Locale;

pub fn branch(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Branch",
        Locale::Zh => "分支",
    }
}

pub fn words(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Words",
        Locale::Zh => "字数",
    }
}

pub fn lines(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Lines",
        Locale::Zh => "行数",
    }
}

pub fn col(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Col",
        Locale::Zh => "列",
    }
}

pub fn chars(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Chars",
        Locale::Zh => "字符",
    }
}

pub fn toggle_status_details(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Toggle status details",
        Locale::Zh => "切换状态详情",
    }
}

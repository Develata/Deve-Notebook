//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! Bottom bar playback labels.

use super::super::Locale;

pub fn first(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "First",
        Locale::Zh => "最前",
    }
}

pub fn prev(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Prev",
        Locale::Zh => "上一步",
    }
}

pub fn next(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Next",
        Locale::Zh => "下一步",
    }
}

pub fn latest(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Latest",
        Locale::Zh => "最新",
    }
}

pub fn time_travel(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Time Travel",
        Locale::Zh => "时间回放",
    }
}

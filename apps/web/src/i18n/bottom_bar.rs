// apps\web\src\i18n
//! # I18n Bottom Bar Module (底部栏翻译)

use super::Locale;

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

pub fn ready(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Ready",
        Locale::Zh => "就绪",
    }
}

pub fn syncing(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Syncing...",
        Locale::Zh => "同步中...",
    }
}

pub fn offline(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Offline",
        Locale::Zh => "离线",
    }
}

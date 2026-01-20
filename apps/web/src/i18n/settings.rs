// apps\web\src\i18n
//! # I18n Settings Module (设置翻译)

use super::Locale;

pub fn title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Settings",
        Locale::Zh => "设置",
    }
}

pub fn close(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Close",
        Locale::Zh => "关闭",
    }
}

pub fn about(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "About",
        Locale::Zh => "关于",
    }
}

pub fn version(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Version",
        Locale::Zh => "版本",
    }
}

pub fn language(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Language",
        Locale::Zh => "语言",
    }
}

pub fn hybrid_mode(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Hybrid Editing",
        Locale::Zh => "混合编辑",
    }
}

pub fn hybrid_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Hide Markdown syntax while reading",
        Locale::Zh => "阅读时隐藏 Markdown 语法",
    }
}

pub fn coming_soon(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Coming in Phase 6",
        Locale::Zh => "将在 Phase 6 推出",
    }
}

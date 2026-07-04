//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Type a command...",
        Locale::Zh => "输入命令...",
    }
}

pub fn dialog_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Command Palette",
        Locale::Zh => "命令面板",
    }
}

pub fn no_results(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No results found.",
        Locale::Zh => "未找到结果。",
    }
}

pub fn open_settings(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open Settings",
        Locale::Zh => "打开设置",
    }
}

pub fn toggle_language(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Toggle Language",
        Locale::Zh => "切换语言",
    }
}

pub fn toggle_sidebar(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Toggle Sidebar",
        Locale::Zh => "切换侧边栏",
    }
}

pub fn open_document(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open Document",
        Locale::Zh => "打开文档",
    }
}

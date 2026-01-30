// apps\web\src\i18n
//! # I18n Command Palette Module (命令面板翻译)

#![allow(dead_code)] // 翻译字符串按需使用

use super::Locale;

pub fn placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Type a command...",
        Locale::Zh => "输入命令...",
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
        Locale::En => "Open Settings (config)",
        Locale::Zh => "打开设置 (config)",
    }
}

pub fn toggle_language(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Toggle Language",
        Locale::Zh => "切换语言",
    }
}

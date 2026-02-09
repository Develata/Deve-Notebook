// apps\web\src\i18n
//! # I18n Header Module (头部翻译)
//!
//! 包含顶部导航栏相关的翻译字符串。

#![allow(dead_code)] // 翻译字符串按需使用

use super::Locale;

pub fn settings(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Settings",
        Locale::Zh => "设置",
    }
}

pub fn home(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Home",
        Locale::Zh => "首页",
    }
}

pub fn open(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open Index",
        Locale::Zh => "打开目录",
    }
}

pub fn command(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Command Palette",
        Locale::Zh => "命令面板",
    }
}

pub fn file_tree(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "File tree",
        Locale::Zh => "文件树",
    }
}

pub fn toggle_outline(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Toggle Outline",
        Locale::Zh => "切换大纲",
    }
}

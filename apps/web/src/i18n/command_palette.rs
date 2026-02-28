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

pub fn open_document(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open Document",
        Locale::Zh => "打开文档",
    }
}

pub fn switch_peer(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "P2P: Switch to Peer",
        Locale::Zh => "P2P: 切换到节点",
    }
}

pub fn establish_branch(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "P2P: Establish Branch",
        Locale::Zh => "P2P: 建立分支",
    }
}

pub fn merge_peer(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "P2P: Merge Peer",
        Locale::Zh => "P2P: 合并当前节点",
    }
}

pub fn toggle_ai_chat(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI: Toggle Chat Panel",
        Locale::Zh => "AI: 切换聊天面板",
    }
}

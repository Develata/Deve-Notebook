// apps\web\src\i18n
//! plan_ref:
//!   - 11_i18n#i18n-keys-reference
//!
//! # I18n Search Module (搜索翻译)

use super::Locale;

pub fn placeholder_command(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Search commands...",
        Locale::Zh => "搜索命令...",
    }
}

pub fn placeholder_branch(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Switch branch...",
        Locale::Zh => "切换分支...",
    }
}

pub fn placeholder_file(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "folder/.../file(.md)",
        Locale::Zh => "文件夹/.../文件(.md)",
    }
}

pub fn placeholder_full_text(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Search note contents...",
        Locale::Zh => "搜索笔记正文...",
    }
}

pub fn full_text_match(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Full-text match",
        Locale::Zh => "全文匹配",
    }
}

pub fn failed(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Search failed",
        Locale::Zh => "搜索失败",
    }
}

pub fn unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Search unavailable",
        Locale::Zh => "搜索不可用",
    }
}

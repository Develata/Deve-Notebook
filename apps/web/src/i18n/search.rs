// apps\web\src\i18n
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

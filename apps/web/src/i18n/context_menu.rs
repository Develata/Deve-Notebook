// apps\web\src\i18n
//! # I18n Context Menu Module (右键菜单翻译)
//!
//! 文件树右键菜单项的翻译字符串。

use super::Locale;

pub fn rename(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Rename",
        Locale::Zh => "重命名",
    }
}

pub fn copy(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Copy",
        Locale::Zh => "复制",
    }
}

pub fn open_in_new_window(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open in New Window",
        Locale::Zh => "在新窗口中打开",
    }
}

pub fn move_to(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Move to...",
        Locale::Zh => "移动到...",
    }
}

pub fn delete(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Delete",
        Locale::Zh => "删除",
    }
}

// apps\web\src\i18n
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
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

pub fn copy_absolute_path(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Copy Absolute Path",
        Locale::Zh => "复制绝对路径",
    }
}

pub fn reveal_in_system_explorer(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Show in System File Manager",
        Locale::Zh => "在系统资源管理器中显示",
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

pub fn export_pdf(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Export PDF",
        Locale::Zh => "导出 PDF",
    }
}

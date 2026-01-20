// apps\web\src\i18n
//! # I18n Common Module (通用翻译)
//!
//! 包含跨模块使用的通用翻译字符串。

use super::Locale;

/// 创建
pub fn create(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Create",
        Locale::Zh => "创建",
    }
}

/// 新建文件
pub fn new_file(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "New File",
        Locale::Zh => "新建文件",
    }
}

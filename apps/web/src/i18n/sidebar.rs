// apps\web\src\i18n
//! # I18n Sidebar Module (侧边栏翻译)

use super::Locale;

pub fn no_docs(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No documents found",
        Locale::Zh => "暂无文档",
    }
}

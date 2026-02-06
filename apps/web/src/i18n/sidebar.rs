// apps\web\src\i18n
//! # I18n Sidebar Module (侧边栏翻译)

use super::Locale;

pub fn no_docs(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No documents found",
        Locale::Zh => "暂无文档",
    }
}

pub fn files(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Files",
        Locale::Zh => "文件",
    }
}

pub fn no_docs_yet(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No documents yet",
        Locale::Zh => "暂无文档",
    }
}

pub fn create_first_note(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Create your first note",
        Locale::Zh => "创建你的第一条笔记",
    }
}

pub fn new_note(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "New Note",
        Locale::Zh => "新建笔记",
    }
}

pub fn close_file_tree(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Close file tree",
        Locale::Zh => "关闭文件树",
    }
}

pub fn outline(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Outline",
        Locale::Zh => "大纲",
    }
}

pub fn close_outline(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Close outline",
        Locale::Zh => "关闭大纲",
    }
}

pub fn outline_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Outline unavailable",
        Locale::Zh => "大纲不可用",
    }
}

pub fn no_headings_found(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No headings found",
        Locale::Zh => "未找到标题",
    }
}

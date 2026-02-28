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

pub fn read_only_mode(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Read-Only Mode",
        Locale::Zh => "只读模式",
    }
}

pub fn select_document(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Select a document",
        Locale::Zh => "请选择文档",
    }
}

pub fn tab(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Tab",
        Locale::Zh => "制表",
    }
}

pub fn heading(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Heading",
        Locale::Zh => "标题",
    }
}

pub fn list(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "List",
        Locale::Zh => "列表",
    }
}

pub fn task(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Task",
        Locale::Zh => "任务",
    }
}

pub fn bold(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Bold",
        Locale::Zh => "加粗",
    }
}

pub fn italic(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Italic",
        Locale::Zh => "斜体",
    }
}

pub fn code(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Code",
        Locale::Zh => "代码",
    }
}

pub fn undo(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Undo",
        Locale::Zh => "撤销",
    }
}

pub fn read_only_watermark(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "READ ONLY",
        Locale::Zh => "只读",
    }
}

pub fn spectator_status(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Spectator Mode - Read Only",
        Locale::Zh => "旁观者模式 - 只读",
    }
}

pub fn select_document_to_edit(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Select a document to edit",
        Locale::Zh => "请选择一个文档进行编辑",
    }
}

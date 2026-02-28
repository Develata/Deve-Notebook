// apps\web\src\i18n
//! # I18n Sidebar Module (侧边栏翻译)

use super::Locale;

pub fn no_docs(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No documents found",
        Locale::Zh => "暂无文档",
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

pub fn explorer(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Explorer",
        Locale::Zh => "资源管理器",
    }
}

pub fn search(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Search",
        Locale::Zh => "搜索",
    }
}

pub fn source_control(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Source Control",
        Locale::Zh => "源代码管理",
    }
}

pub fn extensions(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Extensions",
        Locale::Zh => "扩展",
    }
}

pub fn more(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "More",
        Locale::Zh => "更多",
    }
}

pub fn more_actions(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "More Actions",
        Locale::Zh => "更多操作",
    }
}

pub fn knowledge_base(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Knowledge Base",
        Locale::Zh => "知识库",
    }
}

pub fn local_branch(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Local",
        Locale::Zh => "本地",
    }
}

pub fn local_master_branch(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Local (Master)",
        Locale::Zh => "本地 (主分支)",
    }
}

// apps\web\src\i18n
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! # I18n Sidebar Module (侧边栏翻译)

use super::Locale;

pub fn no_docs(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No documents found",
        Locale::Zh => "暂无文档",
    }
}

pub fn close_sidebar(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Close sidebar",
        Locale::Zh => "关闭侧栏",
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

pub fn empty_outline_heading(locale: Locale, line: usize) -> String {
    match locale {
        Locale::En => format!("Empty heading on line {line}"),
        Locale::Zh => format!("第 {line} 行空标题"),
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

pub fn external_changes(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "External Changes",
        Locale::Zh => "外部修改",
    }
}

pub fn remote_import(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote Import",
        Locale::Zh => "远程导入",
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

pub fn extensions_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Extension system under development",
        Locale::Zh => "扩展系统开发中",
    }
}

pub fn switch_repository(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Switch Repository",
        Locale::Zh => "切换仓库",
    }
}

pub fn new_repository(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "New Repository",
        Locale::Zh => "新增存储库",
    }
}

pub fn create_repository(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Create Repository",
        Locale::Zh => "创建存储库",
    }
}

pub fn repository_name_placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Repository name",
        Locale::Zh => "存储库名称",
    }
}

pub fn repository_actions(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Repository actions",
        Locale::Zh => "存储库操作",
    }
}

pub fn rename_repository(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Rename",
        Locale::Zh => "重命名",
    }
}

pub fn remove_repository(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remove",
        Locale::Zh => "移除",
    }
}

pub fn switch_branch_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Switch Branch (Ctrl+Shift+K)",
        Locale::Zh => "切换分支 (Ctrl+Shift+K)",
    }
}

pub fn new_doc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "New Doc",
        Locale::Zh => "新建文档",
    }
}

pub fn read_badge(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "READ",
        Locale::Zh => "只读",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        close_outline, close_sidebar, empty_outline_heading, no_headings_found, outline_unavailable,
    };
    use crate::i18n::Locale;

    #[test]
    fn mobile_i18n_sidebar_drawer_copy_has_facade_keys() {
        assert_eq!(close_sidebar(Locale::En), "Close sidebar");
        assert_eq!(close_sidebar(Locale::Zh), "关闭侧栏");
        assert_eq!(close_outline(Locale::En), "Close outline");
        assert_eq!(outline_unavailable(Locale::En), "Outline unavailable");
        assert_eq!(no_headings_found(Locale::En), "No headings found");
        assert_eq!(
            empty_outline_heading(Locale::En, 3),
            "Empty heading on line 3"
        );
        assert_eq!(empty_outline_heading(Locale::Zh, 3), "第 3 行空标题");
    }
}

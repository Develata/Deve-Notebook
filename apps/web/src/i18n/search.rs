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

pub fn unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Search unavailable",
        Locale::Zh => "搜索不可用",
    }
}

pub fn command_detail(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Command",
        Locale::Zh => "命令",
    }
}

pub fn current_branch(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Current Branch",
        Locale::Zh => "当前分支",
    }
}

pub fn remote_branch(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote Branch",
        Locale::Zh => "远程分支",
    }
}

pub fn create_or_open(locale: Locale, path: &str) -> String {
    match locale {
        Locale::En => format!("Create/Open '{}'", path),
        Locale::Zh => format!("创建/打开 '{}'", path),
    }
}

pub fn file_op_detail(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "FileOp",
        Locale::Zh => "文件操作",
    }
}

pub fn group_detail(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Group",
        Locale::Zh => "分组",
    }
}

pub fn error_detail(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Error",
        Locale::Zh => "错误",
    }
}

pub fn move_file_op(locale: Locale, src: &str, dst: &str) -> String {
    match locale {
        Locale::En => format!("Move: {} -> {}", src, dst),
        Locale::Zh => format!("移动：{} -> {}", src, dst),
    }
}

pub fn copy_file_op(locale: Locale, src: &str, dst: &str) -> String {
    match locale {
        Locale::En => format!("Copy: {} -> {}", src, dst),
        Locale::Zh => format!("复制：{} -> {}", src, dst),
    }
}

pub fn remove_file_op(locale: Locale, path: &str) -> String {
    match locale {
        Locale::En => format!("Remove: {}", path),
        Locale::Zh => format!("删除：{}", path),
    }
}

pub fn recent_group(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Recent",
        Locale::Zh => "最近",
    }
}

pub fn all_group(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "All",
        Locale::Zh => "全部",
    }
}

pub fn directory_detail(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Directory",
        Locale::Zh => "目录",
    }
}

pub fn paths_with_spaces_must_be_quoted(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Paths with spaces must be quoted",
        Locale::Zh => "包含空格的路径必须加引号",
    }
}

pub fn unclosed_quote(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unclosed quote",
        Locale::Zh => "引号未闭合",
    }
}

pub fn remove_usage(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Usage: >rm <path>",
        Locale::Zh => "用法：>rm <path>",
    }
}

// apps/web/src/i18n/common.rs
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! # I18n Common Module (通用翻译)
//!
//! 包含跨模块使用的通用翻译字符串。

use super::Locale;

mod native;
mod pending;

pub use native::*;
pub use pending::*;

/// 创建
pub fn create(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Create",
        Locale::Zh => "创建",
    }
}

pub fn cancel(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Cancel",
        Locale::Zh => "取消",
    }
}

pub fn confirm(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Confirm",
        Locale::Zh => "确认",
    }
}

pub fn close(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Close",
        Locale::Zh => "关闭",
    }
}

/// 新建文件
pub fn new_file(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "New File",
        Locale::Zh => "新建文件",
    }
}

pub fn status(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Status",
        Locale::Zh => "状态",
    }
}

pub fn read_only_mode(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Read-Only Mode",
        Locale::Zh => "只读模式",
    }
}

pub fn pin(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Pin",
        Locale::Zh => "固定",
    }
}

pub fn unpin(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unpin",
        Locale::Zh => "取消固定",
    }
}

pub fn document_tab(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Document tab",
        Locale::Zh => "文档标签页",
    }
}

pub fn diff_tab(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Diff tab",
        Locale::Zh => "差异标签页",
    }
}

pub fn close_tab(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Close tab",
        Locale::Zh => "关闭标签页",
    }
}

pub fn open_tabs(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Open tabs",
        Locale::Zh => "已打开标签页",
    }
}

pub fn switch_open_tabs(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Switch open tabs",
        Locale::Zh => "切换已打开标签页",
    }
}

pub fn documents(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Documents",
        Locale::Zh => "文档",
    }
}

pub fn diffs(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Diffs",
        Locale::Zh => "差异",
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

pub fn indent(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Indent",
        Locale::Zh => "缩进",
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

pub fn redo(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Redo",
        Locale::Zh => "重做",
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

pub fn disconnected(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Disconnected",
        Locale::Zh => "已断开连接",
    }
}

pub fn reconnecting(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Reconnecting...",
        Locale::Zh => "正在重连...",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_actions_are_localized() {
        assert_eq!(pin(Locale::En), "Pin");
        assert_eq!(pin(Locale::Zh), "固定");
        assert_eq!(unpin(Locale::En), "Unpin");
        assert_eq!(unpin(Locale::Zh), "取消固定");
        assert_eq!(document_tab(Locale::Zh), "文档标签页");
        assert_eq!(diff_tab(Locale::En), "Diff tab");
        assert_eq!(diff_tab(Locale::Zh), "差异标签页");
        assert_eq!(close(Locale::Zh), "关闭");
        assert_eq!(close_tab(Locale::Zh), "关闭标签页");
        assert_eq!(indent(Locale::En), "Indent");
        assert_eq!(indent(Locale::Zh), "缩进");
        assert_eq!(open_tabs(Locale::En), "Open tabs");
        assert_eq!(open_tabs(Locale::Zh), "已打开标签页");
        assert_eq!(switch_open_tabs(Locale::Zh), "切换已打开标签页");
        assert_eq!(documents(Locale::Zh), "文档");
        assert_eq!(diffs(Locale::En), "Diffs");
    }
}

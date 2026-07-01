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
mod tabs;

pub use native::*;
pub use pending::*;
pub use tabs::*;

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
    }
}

// apps/web/src/i18n/common.rs
//! plan_ref:
//!   - 11_i18n#i18n-keys-reference
//!
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

pub fn native_bootstrap_invalid_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native bootstrap invalid",
        Locale::Zh => "原生启动参数无效",
    }
}

pub fn native_bootstrap_invalid_body(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "The native shell provided an invalid local endpoint. Restart the app or service."
        }
        Locale::Zh => "原生外壳提供了无效的本地端点。请重启应用或本地服务。",
    }
}

pub fn native_session_pending_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native session pending",
        Locale::Zh => "等待原生会话",
    }
}

pub fn native_session_pending_body(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Waiting for the native shell to bind the local session before loading the workspace."
        }
        Locale::Zh => "正在等待原生外壳绑定本地会话，然后再加载工作区。",
    }
}

pub fn native_service_offline_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native service offline",
        Locale::Zh => "原生服务离线",
    }
}

pub fn native_service_offline_body(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "The embedded local service is unavailable. Restart the native service from the shell."
        }
        Locale::Zh => "嵌入式本地服务不可用。请从原生外壳重启本地服务。",
    }
}

pub fn native_reprobe_required_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native reprobe required",
        Locale::Zh => "需要重新探测原生会话",
    }
}

pub fn native_reprobe_required_body(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "The app returned from background and must revalidate session and workspace state before writing."
        }
        Locale::Zh => "应用从后台恢复后，需要重新验证会话与工作区状态才能写入。",
    }
}

pub fn pending_navigation_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unconfirmed Local Edits",
        Locale::Zh => "存在未确认本地写入",
    }
}

pub fn pending_navigation_body(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "The current document still has local edits waiting for server acknowledgement."
        }
        Locale::Zh => "当前文档仍有等待服务端确认的本地写入。",
    }
}

pub fn pending_navigation_note(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Continuing only leaves this view. It does not mean those edits were committed."
        }
        Locale::Zh => "继续只会离开当前视图，不代表这些写入已经提交。",
    }
}

pub fn pending_navigation_destination(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Destination",
        Locale::Zh => "目标",
    }
}

pub fn pending_navigation_doc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Another document",
        Locale::Zh => "另一篇文档",
    }
}

pub fn pending_navigation_repo(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Another repository",
        Locale::Zh => "另一个仓库",
    }
}

pub fn pending_navigation_branch(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Another branch",
        Locale::Zh => "另一个分支",
    }
}

pub fn pending_navigation_home(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Home",
        Locale::Zh => "首页",
    }
}

pub fn pending_navigation_cancel(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Cancel",
        Locale::Zh => "取消",
    }
}

pub fn pending_navigation_continue(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Leave View",
        Locale::Zh => "继续切换",
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

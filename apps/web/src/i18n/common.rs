// apps/web/src/i18n/common.rs
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
        Locale::En => "Reconnecting to server... please wait.",
        Locale::Zh => "正在重连服务器...请稍候。",
    }
}

// === Login related ===
pub fn app_name(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Deve Notebook",
        Locale::Zh => "Deve 笔记",
    }
}

pub fn login_subtitle(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sign in to continue",
        Locale::Zh => "登录以继续",
    }
}

pub fn username(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Username",
        Locale::Zh => "用户名",
    }
}

pub fn password(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Password",
        Locale::Zh => "密码",
    }
}

pub fn username_placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Enter username",
        Locale::Zh => "输入用户名",
    }
}

pub fn password_placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Enter password",
        Locale::Zh => "输入密码",
    }
}

pub fn login_button(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sign In",
        Locale::Zh => "登录",
    }
}

pub fn logging_in(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Signing in...",
        Locale::Zh => "登录中...",
    }
}

pub fn login_failed(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Invalid username or password",
        Locale::Zh => "用户名或密码错误",
    }
}

pub fn login_error(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Login error",
        Locale::Zh => "登录错误",
    }
}

// apps\web\src\i18n
//! # I18n Chat Module (聊天翻译)

use super::Locale;

pub fn panel_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI Assistant",
        Locale::Zh => "AI 助手",
    }
}

pub fn mode_plan(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "PLAN",
        Locale::Zh => "计划",
    }
}

pub fn mode_build(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "BUILD",
        Locale::Zh => "执行",
    }
}

pub fn you(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "You",
        Locale::Zh => "你",
    }
}

pub fn assistant(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Assistant",
        Locale::Zh => "助手",
    }
}

pub fn thinking(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Thinking...",
        Locale::Zh => "思考中...",
    }
}

pub fn input_placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Ask anything... (Shift+Enter for newline)",
        Locale::Zh => "输入你的问题...（Shift+Enter 换行）",
    }
}

pub fn markdown_supported(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Markdown supported",
        Locale::Zh => "支持 Markdown",
    }
}

pub fn send(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Send",
        Locale::Zh => "发送",
    }
}

pub fn send_failed(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Send failed",
        Locale::Zh => "发送失败",
    }
}

pub fn retry(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Retry",
        Locale::Zh => "重试",
    }
}

pub fn loading(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Loading response...",
        Locale::Zh => "响应加载中...",
    }
}

pub fn empty_brand(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Deve-Note AI",
        Locale::Zh => "Deve-Note AI",
    }
}

pub fn try_these(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Try these",
        Locale::Zh => "试试这些",
    }
}

pub fn tip(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI can stage and commit after review",
        Locale::Zh => "AI 可在审阅后辅助暂存与提交",
    }
}

pub fn apply(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Apply",
        Locale::Zh => "应用",
    }
}

pub fn toggle_mobile_chat(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Toggle AI chat",
        Locale::Zh => "切换 AI 聊天",
    }
}

pub fn mobile_chip(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI",
        Locale::Zh => "AI",
    }
}

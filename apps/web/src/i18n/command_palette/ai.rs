//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn toggle_ai_chat(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI: Toggle Chat Panel",
        Locale::Zh => "AI: 切换聊天面板",
    }
}

pub fn ai_retry_last_request(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI: Retry Last Request",
        Locale::Zh => "AI: 重试上次请求",
    }
}

pub fn ai_switch_backend(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI: Switch Backend",
        Locale::Zh => "AI: 切换后端",
    }
}

pub fn ai_switch_plan(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI: Switch to PLAN Mode",
        Locale::Zh => "AI: 切换到 PLAN 模式",
    }
}

pub fn ai_switch_build(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI: Switch to BUILD Mode",
        Locale::Zh => "AI: 切换到 BUILD 模式",
    }
}

pub fn ai_retry_panel_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unavailable: retry is scoped to the open AI Chat panel",
        Locale::Zh => "不可用：重试只在已打开的 AI Chat 面板内生效",
    }
}

pub fn ai_backend_settings_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unavailable: backend switching is gated by Settings and server capabilities",
        Locale::Zh => "不可用：后端切换由 Settings 与服务端能力门禁控制",
    }
}

pub fn ai_slash_mode_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unavailable: use /plan or /build in AI Chat",
        Locale::Zh => "不可用：请在 AI Chat 中使用 /plan 或 /build",
    }
}

//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 15_settings#configuration-settings
//!   - 15_settings#browser-ui-prefs

use crate::i18n::Locale;

pub fn ai_backend(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI Backend",
        Locale::Zh => "AI 后端",
    }
}

pub fn ai_backend_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native: built-in read-first chat. Trusted CLI: external advanced bridge.",
        Locale::Zh => "原生: 内置只读优先聊天。受信任 CLI: 外部高级桥接。",
    }
}

pub fn native_backend(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native",
        Locale::Zh => "原生",
    }
}

pub fn trusted_cli_backend(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Trusted CLI",
        Locale::Zh => "受信任 CLI",
    }
}

pub fn ai_chat_panel(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "AI Chat Panel",
        Locale::Zh => "AI Chat 面板",
    }
}

pub fn ai_chat_panel_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Show or hide the chat surface without changing the AI backend.",
        Locale::Zh => "显示或隐藏聊天面板，不改变 AI 后端配置。",
    }
}

pub fn show_ai_chat(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Show AI Chat",
        Locale::Zh => "显示 AI Chat",
    }
}

pub fn hide_ai_chat(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Hide AI Chat",
        Locale::Zh => "隐藏 AI Chat",
    }
}

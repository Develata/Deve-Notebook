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

macro_rules! ai_copy {
    ($name:ident, $en:literal, $zh:literal) => {
        pub fn $name(locale: Locale) -> &'static str {
            match locale {
                Locale::En => $en,
                Locale::Zh => $zh,
            }
        }
    };
}

ai_copy!(ai_provider_title, "Native AI Provider", "原生 AI Provider");
ai_copy!(
    ai_provider_desc,
    "Server-owned provider settings. The API key is write-only and is never shown again.",
    "由服务端持有 Provider 配置。API key 只写入，保存后不会再次显示。"
);
ai_copy!(
    ai_provider_environment_managed,
    "Managed by deployment environment; restart the service after changing .env.",
    "由部署环境管理；修改 .env 后请重启服务。"
);
ai_copy!(ai_provider_protocol, "Protocol", "协议");
ai_copy!(ai_provider_model, "Model", "模型");
ai_copy!(ai_provider_base_url, "Base URL", "Base URL");
ai_copy!(
    ai_provider_max_tokens,
    "Max output tokens",
    "最大输出 tokens"
);
ai_copy!(ai_provider_api_key, "API key", "API key");
ai_copy!(
    ai_provider_key_keep,
    "Configured — leave blank to keep",
    "已配置——留空则保留"
);
ai_copy!(ai_provider_key_missing, "Not configured", "尚未配置");
ai_copy!(
    ai_provider_save,
    "Save provider settings",
    "保存 Provider 设置"
);
ai_copy!(
    ai_provider_clear_key,
    "Clear key when saved",
    "保存时清除 key"
);
ai_copy!(ai_provider_undo_clear_key, "Undo key clear", "撤销清除 key");
ai_copy!(
    ai_provider_clear_pending,
    "The saved key will be removed only after you save these settings.",
    "仅在保存这些设置后，已保存的 key 才会被删除。"
);
ai_copy!(ai_provider_saving, "Saving…", "正在保存…");
ai_copy!(
    ai_provider_saved,
    "Provider settings saved.",
    "Provider 设置已保存。"
);
ai_copy!(
    ai_provider_revision_conflict,
    "Settings changed elsewhere. Reopen Settings and try again.",
    "设置已在其他位置更改，请重新打开 Settings 后再试。"
);
ai_copy!(
    ai_provider_invalid,
    "Provider settings are invalid.",
    "Provider 设置无效。"
);
ai_copy!(
    ai_provider_unavailable,
    "Provider settings are unavailable.",
    "Provider 设置暂不可用。"
);

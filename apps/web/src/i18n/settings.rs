// apps\web\src\i18n
//! # I18n Settings Module (设置翻译)

use super::Locale;

pub fn title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Settings",
        Locale::Zh => "设置",
    }
}

pub fn close(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Close",
        Locale::Zh => "关闭",
    }
}

pub fn about(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "About",
        Locale::Zh => "关于",
    }
}

pub fn version(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Version",
        Locale::Zh => "版本",
    }
}

pub fn language(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Language",
        Locale::Zh => "语言",
    }
}

pub fn hybrid_mode(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Hybrid Editing",
        Locale::Zh => "混合编辑",
    }
}

pub fn hybrid_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Hide Markdown syntax while reading",
        Locale::Zh => "阅读时隐藏 Markdown 语法",
    }
}

pub fn coming_soon(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Coming in Phase 6",
        Locale::Zh => "将在 Phase 6 推出",
    }
}

pub fn sync_mode(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sync Mode",
        Locale::Zh => "同步模式",
    }
}

pub fn sync_mode_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Auto: instant sync. Manual: review before merge.",
        Locale::Zh => "自动: 实时同步。手动: 合并前审查。",
    }
}

pub fn auto_mode(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Auto",
        Locale::Zh => "自动",
    }
}

pub fn manual_mode(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Manual",
        Locale::Zh => "手动",
    }
}

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

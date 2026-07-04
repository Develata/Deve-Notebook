//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 15_settings#configuration-settings
//!   - 15_settings#browser-ui-prefs

use crate::i18n::Locale;

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

pub fn english_language_label() -> &'static str {
    "English"
}

pub fn chinese_language_label() -> &'static str {
    "中文"
}

pub fn current_boundary(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Current Settings Boundary",
        Locale::Zh => "当前设置边界",
    }
}

pub fn current_boundary_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "This panel applies runtime/local UI feedback. Persistent runtime config is still updated with `deve config set` in config.toml."
        }
        Locale::Zh => {
            "此面板只提供运行时/本地 UI 反馈。持久运行时配置仍通过 `deve config set` 写入 config.toml。"
        }
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
        Locale::En => "Future setting: not available in the current release",
        Locale::Zh => "未来设置：当前版本不可用",
    }
}

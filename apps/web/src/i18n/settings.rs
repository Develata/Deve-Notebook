// apps\web\src\i18n
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! # I18n Settings Module (设置翻译)

use super::Locale;

mod local_prefs;

pub use local_prefs::*;

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

pub fn backend_section(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Backend",
        Locale::Zh => "后端",
    }
}

pub fn backend_section_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native shells can use a local service or a validated remote origin.",
        Locale::Zh => "Native 壳层可以使用本机服务或已校验的远端 origin。",
    }
}

pub fn local_backend_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Local Backend",
        Locale::Zh => "本地后端",
    }
}

pub fn remote_backend_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote Backend",
        Locale::Zh => "远端后端",
    }
}

pub fn remote_backend_url_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote HTTPS origin",
        Locale::Zh => "远端 HTTPS origin",
    }
}

pub fn validate_and_save_remote(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Validate & Save",
        Locale::Zh => "校验并保存",
    }
}

pub fn validating_remote_backend(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Validating remote backend...",
        Locale::Zh => "正在校验远端后端...",
    }
}

pub fn remote_backend_saved(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote backend saved. The native shell will load that origin.",
        Locale::Zh => "远端后端已保存。Native 壳层将加载该 origin。",
    }
}

pub fn remote_backend_requires_validation(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Remote URL must pass native validation before it can be saved.",
        Locale::Zh => "远端 URL 必须先通过 native 校验才能保存。",
    }
}

pub fn native_backend_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Native-only setting unavailable in a regular browser.",
        Locale::Zh => "普通浏览器中不可用：这是 native-only 设置。",
    }
}

pub fn use_local_backend(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Use Local Backend",
        Locale::Zh => "使用本地后端",
    }
}

pub fn local_backend_switching(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Switching to local backend...",
        Locale::Zh => "正在切换到本地后端...",
    }
}

pub fn local_backend_saved(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Local backend saved. The native shell will restart the local service.",
        Locale::Zh => "本地后端已保存。Native 壳层将重启本机服务。",
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

#[cfg(test)]
mod tests {
    use super::{
        Locale, chinese_language_label, current_boundary_desc, english_language_label,
        native_backend_unavailable, remote_backend_requires_validation,
    };

    #[test]
    fn boundary_copy_mentions_config_toml_and_cli_set() {
        for locale in [Locale::En, Locale::Zh] {
            let text = current_boundary_desc(locale);
            assert!(text.contains("config.toml"));
            assert!(text.contains("deve config set"));
        }
    }

    #[test]
    fn language_buttons_use_self_labels() {
        assert_eq!(english_language_label(), "English");
        assert_eq!(chinese_language_label(), "中文");
    }

    #[test]
    fn reserved_setting_copy_marks_future_boundary() {
        assert!(super::coming_soon(Locale::En).contains("Future setting"));
        assert!(super::coming_soon(Locale::Zh).contains("未来设置"));
    }

    #[test]
    fn native_backend_copy_marks_native_only_and_validation_boundary() {
        assert!(native_backend_unavailable(Locale::En).contains("Native-only"));
        assert!(native_backend_unavailable(Locale::Zh).contains("native-only"));
        assert!(remote_backend_requires_validation(Locale::En).contains("validation"));
        assert!(remote_backend_requires_validation(Locale::Zh).contains("校验"));
    }
}

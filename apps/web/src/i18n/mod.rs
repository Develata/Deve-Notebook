// apps\web\src\i18n
//! plan_ref:
//!   - 13_i18n#i18n-facade-contract
//!   - 13_i18n#i18n-resource-management
//!
//! # Internationalization Module (国际化模块)
//!
//! **架构作用**:
//! 管理应用程序的语言设置和翻译字符串。
//! 提供简单的 `Locale` 枚举和嵌套模块结构来组织 UI 文本。
//!
//! **模块结构**:
//! - `common`: 通用翻译 (Create, New File, etc.)
//! - `header`: 顶部导航栏翻译
//! - `sidebar`: 侧边栏翻译
//! - `settings`: 设置面板翻译
//! - `bottom_bar`: 底部状态栏翻译
//! - `playback`: 回放控制翻译
//! - `command_palette`: 命令面板翻译
//! - `search`: 搜索框翻译
//! - `source_control`: 版本控制面板翻译
//! - `source_control_history`: 版本控制历史面板翻译
//! - `time`: 时间与相对时间翻译
//! - `write_gate`: 写入门禁与受限操作提示
//! - `login`: 登录页与认证错误翻译

pub mod bottom_bar;
pub mod chat;
pub mod command_palette;
pub mod common;
pub mod context_menu;
pub mod dashboard;
pub mod diff;
pub mod editor_sync;
pub mod extensions;
pub mod external_changes;
pub mod header;
pub mod js_bridge;
pub mod login;
pub mod merge;
pub mod playback;
pub mod search;
pub mod server_error;
pub mod settings;
pub mod sidebar;
pub mod source_control;
pub mod source_control_git;
pub mod source_control_graph;
pub mod source_control_history;
pub mod source_control_native;
pub mod source_control_remote_projection;
pub mod time;
pub mod workspace_ingestion;
pub mod write_gate;

use crate::storage::prefs::{read_pref, write_pref};

pub use js_bridge::publish_browser_i18n;

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const LOCALE_STORAGE_KEY: &str = "deve.ui.locale";

/// 语言枚举
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Locale {
    #[default]
    En,
    Zh,
}

impl Locale {
    /// 切换语言
    pub const fn toggle(&self) -> Self {
        match self {
            Self::En => Self::Zh,
            Self::Zh => Self::En,
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub const fn as_bcp47(self) -> &'static str {
        match self {
            Self::En => "en-US",
            Self::Zh => "zh-CN",
        }
    }

    pub fn from_supported_tag(tag: &str) -> Option<Self> {
        let tag = tag.trim().to_ascii_lowercase();
        if tag == "auto" || tag.is_empty() {
            return None;
        }
        if tag == "en" || tag.starts_with("en-") || tag.starts_with("en_") {
            return Some(Self::En);
        }
        if tag == "zh" || tag.starts_with("zh-") || tag.starts_with("zh_") {
            return Some(Self::Zh);
        }
        None
    }

    pub fn detect(configured: Option<&str>, browser_language: Option<&str>) -> Self {
        configured
            .and_then(Self::from_supported_tag)
            .or_else(|| browser_language.and_then(Self::from_supported_tag))
            .unwrap_or_default()
    }
}

pub fn initial_locale() -> Locale {
    let configured = stored_locale_preference();
    let browser = browser_language();
    Locale::detect(configured.as_deref(), browser.as_deref())
}

pub fn persist_locale_preference(locale: Locale) {
    let _ = write_pref(LOCALE_STORAGE_KEY, locale.as_bcp47());
}

fn stored_locale_preference() -> Option<String> {
    read_pref(LOCALE_STORAGE_KEY)
}

#[cfg(target_arch = "wasm32")]
fn browser_language() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.navigator().language())
        .filter(|language| !language.trim().is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_language() -> Option<String> {
    None
}

/// 应用标题
pub fn app_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Deve-Note",
        Locale::Zh => "Deve-Note",
    }
}

// Re-export for backward compatibility (t::xxx::yyy pattern)
pub mod t {
    pub use super::app_title;
    pub use super::bottom_bar;
    pub use super::chat;
    pub use super::command_palette;
    pub use super::common;
    pub use super::context_menu;
    pub use super::dashboard;
    pub use super::diff;
    pub use super::editor_sync;
    pub use super::extensions;
    pub use super::external_changes;
    pub use super::header;
    pub use super::login;
    pub use super::merge;
    pub use super::playback;
    pub use super::search;
    pub use super::server_error;
    pub use super::settings;
    pub use super::sidebar;
    pub use super::source_control;
    pub use super::time;
    pub use super::workspace_ingestion;
    pub use super::write_gate;
}

#[cfg(test)]
mod tests {
    use super::{LOCALE_STORAGE_KEY, Locale, initial_locale, persist_locale_preference};
    use crate::storage::prefs::remove_pref;

    #[test]
    fn locale_detect_prefers_supported_user_config() {
        assert_eq!(Locale::detect(Some("zh-CN"), Some("en-US")), Locale::Zh);
        assert_eq!(Locale::detect(Some("en"), Some("zh-CN")), Locale::En);
    }

    #[test]
    fn locale_detect_uses_browser_when_config_is_auto_or_missing() {
        assert_eq!(Locale::detect(Some("auto"), Some("zh-Hans-CN")), Locale::Zh);
        assert_eq!(Locale::detect(None, Some("en-GB")), Locale::En);
    }

    #[test]
    fn locale_detect_falls_back_to_english_for_unsupported_tags() {
        assert_eq!(Locale::detect(Some("ja-JP"), Some("fr-FR")), Locale::En);
        assert_eq!(Locale::detect(None, Some("xx-XX")), Locale::En);
    }

    #[test]
    fn locale_preference_uses_ui_prefs_fallback_layer() {
        remove_pref(LOCALE_STORAGE_KEY);
        persist_locale_preference(Locale::Zh);

        assert_eq!(initial_locale(), Locale::Zh);

        remove_pref(LOCALE_STORAGE_KEY);
    }

    #[test]
    fn t_facade_exposes_time_namespace() {
        assert_eq!(super::t::time::just_now(Locale::En), "just now");
        assert_eq!(super::t::time::minutes_ago(Locale::Zh, 2), "2 分钟前");
        assert_eq!(super::t::time::date_locale(Locale::Zh), "zh-CN");
    }
}

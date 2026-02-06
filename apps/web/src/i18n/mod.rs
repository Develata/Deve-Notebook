// apps\web\src\i18n
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

pub mod bottom_bar;
pub mod chat;
pub mod command_palette;
pub mod common;
pub mod header;
pub mod playback;
pub mod search;
pub mod settings;
pub mod sidebar;
pub mod source_control;

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
    pub use super::header;
    pub use super::playback;
    pub use super::search;
    pub use super::settings;
    pub use super::sidebar;
    pub use super::source_control;
}

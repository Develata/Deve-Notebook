// apps\web\src\shortcuts
//! # 全局快捷键 (Global Shortcuts)
//! plan_ref:
//!   - 12_commands#command-palette-shortcuts
//!   - 13_settings#keyboard-shortcuts
//!
//! 定义和处理全局快捷键。
//!
//! ## 当前支持的快捷键
//!
//! - `Ctrl+P`: 打开/关闭文件搜索
//! - `Ctrl+Shift+P`: 打开/关闭命令面板
//! - `Ctrl+Shift+K`: 打开/关闭分支切换器
//! - `Ctrl+L`: 切换语言
//! - `Ctrl+Shift+O`: 打开/关闭大纲
//! - `Ctrl+B`: 打开/关闭侧边栏

#![allow(dead_code)] // 快捷键系统模块预留

mod handlers;
mod search_box;

use super::types::KeyCombo;
use crate::i18n::Locale;
use leptos::prelude::*;
use web_sys::KeyboardEvent;

/// 全局快捷键 ID
pub mod ids {
    /// 打开/关闭文件搜索
    pub const GO_TO_FILE: &str = "global.go_to_file";
    /// 打开/关闭命令面板
    pub const COMMAND_PALETTE: &str = "global.command_palette";
    /// 打开/关闭分支切换器
    pub const BRANCH_SWITCHER: &str = "global.branch_switcher";
    /// 切换语言
    pub const TOGGLE_LANGUAGE: &str = "global.toggle_language";
    /// 打开/关闭大纲
    pub const TOGGLE_OUTLINE: &str = "global.toggle_outline";
    /// 打开/关闭侧边栏
    pub const TOGGLE_SIDEBAR: &str = "global.toggle_sidebar";
}

/// 全局快捷键按键组合
pub mod combos {
    use super::KeyCombo;

    /// Ctrl+P
    pub fn go_to_file() -> KeyCombo {
        KeyCombo::new("p", true, false, false)
    }

    /// Ctrl+Shift+P
    pub fn command_palette() -> KeyCombo {
        KeyCombo::new("p", true, true, false)
    }

    /// Ctrl+Shift+K
    pub fn branch_switcher() -> KeyCombo {
        KeyCombo::new("k", true, true, false)
    }

    /// Ctrl+L
    pub fn toggle_language() -> KeyCombo {
        KeyCombo::new("l", true, false, false)
    }

    /// Ctrl+Shift+O
    pub fn toggle_outline() -> KeyCombo {
        KeyCombo::new("o", true, true, false)
    }

    /// Ctrl+B
    pub fn toggle_sidebar() -> KeyCombo {
        KeyCombo::new("b", true, false, false)
    }
}

/// 创建全局快捷键处理器
///
/// # 参数
///
/// - `show_search`: 搜索框显示状态
/// - `set_show_search`: 设置搜索框显示状态
/// - `search_mode`: 当前搜索模式 ("" 为文件搜索, ">" 为命令)
/// - `set_search_mode`: 设置搜索模式
/// - `locale`: 当前语言状态
/// - `set_outline_visible`: 设置大纲显示状态
/// - `set_sidebar_visible`: 设置侧边栏显示状态
///
/// # 返回
///
/// 返回一个闭包，用于处理键盘事件。
pub fn create_global_shortcut_handler(
    show_search: Signal<bool>,
    set_show_search: WriteSignal<bool>,
    search_mode: Signal<String>,
    set_search_mode: WriteSignal<String>,
    locale: RwSignal<Locale>,
    set_outline_visible: WriteSignal<bool>,
    set_sidebar_visible: WriteSignal<bool>,
) -> impl Fn(KeyboardEvent) + Clone + 'static {
    move |ev: KeyboardEvent| {
        handlers::handle_global_shortcut(
            &ev,
            show_search,
            set_show_search,
            search_mode,
            set_search_mode,
            locale,
            set_outline_visible,
            set_sidebar_visible,
        )
    }
}

/// 处理搜索框内的快捷键（因为搜索框会阻止事件冒泡）
pub fn handle_search_box_keydown(
    ev: &KeyboardEvent,
    set_show: WriteSignal<bool>,
    query: Signal<String>,
    set_query: WriteSignal<String>,
    set_selected_index: WriteSignal<usize>,
    input_ref: NodeRef<leptos::html::Input>,
) {
    search_box::handle_search_box_shortcut(
        ev,
        set_show,
        query,
        set_query,
        set_selected_index,
        input_ref,
    );
}

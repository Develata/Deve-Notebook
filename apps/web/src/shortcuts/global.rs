// apps\web\src\shortcuts
//! # 全局快捷键 (Global Shortcuts)
//!
//! 定义和处理全局快捷键。
//!
//! ## 当前支持的快捷键
//!
//! - `Ctrl+P`: 打开/关闭文件搜索
//! - `Ctrl+Shift+P`: 打开/关闭命令面板
//! - `Ctrl+Shift+K`: 打开/关闭分支切换器

#![allow(dead_code)] // 快捷键系统模块预留

#[path = "global_handlers.rs"]
mod handlers;
#[path = "global_search_box.rs"]
mod search_box;

use super::types::KeyCombo;
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
}

/// 创建全局快捷键处理器
///
/// # 参数
///
/// - `show_search`: 搜索框显示状态
/// - `set_show_search`: 设置搜索框显示状态
/// - `search_mode`: 当前搜索模式 ("" 为文件搜索, ">" 为命令)
/// - `set_search_mode`: 设置搜索模式
///
/// # 返回
///
/// 返回一个闭包，用于处理键盘事件。
pub fn create_global_shortcut_handler(
    show_search: Signal<bool>,
    set_show_search: WriteSignal<bool>,
    search_mode: Signal<String>,
    set_search_mode: WriteSignal<String>,
) -> impl Fn(KeyboardEvent) + Clone + 'static {
    move |ev: KeyboardEvent| {
        handlers::handle_global_shortcut(
            &ev,
            show_search,
            set_show_search,
            search_mode,
            set_search_mode,
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

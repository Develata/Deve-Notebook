//! # 全局快捷键 (Global Shortcuts)
//!
//! 定义和处理全局快捷键。
//!
//! ## 当前支持的快捷键
//!
//! - `Ctrl+P`: 打开/关闭文件搜索
//! - `Ctrl+Shift+P`: 打开/关闭命令面板

use leptos::prelude::*;
use web_sys::KeyboardEvent;
use super::types::KeyCombo;

/// 全局快捷键 ID
pub mod ids {
    /// 打开/关闭文件搜索
    pub const GO_TO_FILE: &str = "global.go_to_file";
    /// 打开/关闭命令面板
    pub const COMMAND_PALETTE: &str = "global.command_palette";
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
        let is_ctrl = ev.meta_key() || ev.ctrl_key();
        let shift = ev.shift_key();
        let key = ev.key().to_lowercase();

        // Ctrl+Shift+P: 命令面板 (必须先检查，因为也包含 Ctrl+P)
        if is_ctrl && shift && key == "p" {
            ev.prevent_default();
            ev.stop_propagation();

            // 如果已打开且为命令模式，则关闭；否则切换到命令模式并打开
            if show_search.get() && search_mode.get() == ">" {
                set_show_search.set(false);
            } else {
                set_search_mode.set(">".to_string());
                set_show_search.set(true);
            }
            return;
        }

        // Ctrl+P: 文件搜索
        if is_ctrl && !shift && key == "p" {
            ev.prevent_default();
            ev.stop_propagation();

            // 如果已打开且为文件模式，则关闭；否则切换到文件模式并打开
            let current_mode = search_mode.get();
            let is_file_mode = current_mode.is_empty();

            if show_search.get() && is_file_mode {
                set_show_search.set(false);
            } else {
                set_search_mode.set(String::new());
                set_show_search.set(true);
            }
        }
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
    let key = ev.key();
    let is_ctrl = ev.ctrl_key() || ev.meta_key();
    let shift = ev.shift_key();
    let key_lower = key.to_lowercase();
    
    // Escape: Close
    if key == "Escape" {
        ev.prevent_default();
        ev.stop_propagation();
        set_show.set(false);
        return;
    }
    
    // Ctrl+P / Shift+P Logic
    if is_ctrl && key_lower == "p" {
         ev.prevent_default();
         ev.stop_propagation();
         
         if shift {
             // Command Palette logic using local query
             if query.get_untracked().starts_with('>') {
                 set_show.set(false);
             } else {
                 set_query.set(">".to_string());
                 set_selected_index.set(0);
                 if let Some(el) = input_ref.get_untracked() { let _ = el.focus(); }
             }
         } else {
             // File Search logic
             let q = query.get_untracked();
             let is_file = !q.starts_with('>') && !q.starts_with('@');
             if is_file {
                 set_show.set(false);
             } else {
                 set_query.set(String::new());
                 set_selected_index.set(0); 
                 if let Some(el) = input_ref.get_untracked() { let _ = el.focus(); }
             }
         }
    }
}

//! # Shortcuts Hook (快捷键钩子)
//!
//! **架构作用**:
//! 处理全局键盘快捷键。
//!
//! **核心功能清单**:
//! - `Ctrl+K`: 切换命令面板。
//! - `Ctrl+L`: 切换语言 (中文/英文)。
//! - `Ctrl+O`: 打开文档。
//! - `Escape`: 关闭模态框。

use leptos::prelude::*;
use web_sys::KeyboardEvent;
use crate::i18n::Locale;

pub fn use_shortcuts(
    locale: RwSignal<Locale>,
    show_search: ReadSignal<bool>,
    set_show_search: WriteSignal<bool>,
    set_search_mode: WriteSignal<String>,
    set_show_open_modal: WriteSignal<bool>
) -> impl Fn(KeyboardEvent) + Clone + 'static {
    move |ev: KeyboardEvent| {
        let is_ctrl = ev.meta_key() || ev.ctrl_key();
        let shift = ev.shift_key();
        let key = ev.key().to_lowercase();
        
        // Ctrl+Shift+P: Command Palette (Force >)
        if is_ctrl && shift && key == "p" {
            ev.prevent_default();
            ev.stop_propagation(); 
            if show_search.get() {
                set_show_search.set(false);
            } else {
                set_search_mode.set(">".to_string());
                set_show_search.set(true);
            }
            return;
        }

        // Ctrl+P: Go to File (Default)
        if is_ctrl && !shift && key == "p" {
            ev.prevent_default();
            ev.stop_propagation(); 
            if show_search.get() {
                 set_show_search.set(false);
            } else {
                 set_search_mode.set(String::new());
                 set_show_search.set(true);
            }
            return;
        }


        
        // Ctrl+L: 切换语言
        if is_ctrl && key == "l" {
             ev.prevent_default();
             ev.stop_propagation();
             locale.update(|l| *l = l.toggle());
        }

        // Ctrl+O: 打开文档模态框 (Legacy Open)
        if is_ctrl && key == "o" {
             ev.prevent_default();
             ev.stop_propagation();
             set_show_open_modal.set(true);
        }
        
        // Escape: 关闭命令面板
        if show_search.get_untracked() && ev.key() == "Escape" {
             set_show_search.set(false);
        }
    }
}

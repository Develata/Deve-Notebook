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
    show_cmd: ReadSignal<bool>,
    set_show_cmd: WriteSignal<bool>,
    set_show_open_modal: WriteSignal<bool>
) -> impl Fn(KeyboardEvent) + Clone + 'static {
    move |ev: KeyboardEvent| {
        let is_ctrl = ev.meta_key() || ev.ctrl_key();
        let key = ev.key().to_lowercase();
        
        // Ctrl+K: 切换命令面板
        if is_ctrl && key == "k" {
            ev.prevent_default();
            ev.stop_propagation(); 
            set_show_cmd.update(|s| *s = !*s);
        }
        
        // Ctrl+L: 切换语言
        if is_ctrl && key == "l" {
             ev.prevent_default();
             ev.stop_propagation();
             locale.update(|l| *l = l.toggle());
        }

        // Ctrl+O: 打开文档模态框
        if is_ctrl && key == "o" {
             ev.prevent_default();
             ev.stop_propagation();
             set_show_open_modal.set(true);
        }
        
        // Escape: 关闭命令面板
        if show_cmd.get_untracked() && ev.key() == "Escape" {
             set_show_cmd.set(false);
        }
    }
}

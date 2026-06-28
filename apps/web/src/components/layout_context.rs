// apps/web/src/components/layout_context.rs
//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! # 布局相关 Context 类型
//!
//! 定义跨组件共享的布局控制上下文。

use crate::components::activity_bar::SidebarView;
use leptos::prelude::*;

/// 搜索控制上下文
///
/// 允许深层组件触发搜索框显示 (如 BranchSwitcher)。
#[derive(Clone, Copy)]
pub struct SearchControl {
    pub set_show: WriteSignal<bool>,
    pub set_mode: WriteSignal<String>,
}

/// AI Chat 面板控制上下文
#[derive(Clone, Copy)]
pub struct ChatControl {
    pub chat_visible: ReadSignal<bool>,
    pub set_chat_visible: WriteSignal<bool>,
}

/// Editor tab limit control context.
#[derive(Clone, Copy)]
pub struct EditorTabLimitControl {
    pub max_document_tabs: ReadSignal<usize>,
    pub set_max_document_tabs: WriteSignal<usize>,
}

/// Outline panel visibility control context.
#[derive(Clone, Copy)]
pub struct OutlineControl {
    pub visible: ReadSignal<bool>,
    pub set_visible: WriteSignal<bool>,
}

/// Sidebar visibility control context.
#[derive(Clone, Copy)]
pub struct SidebarControl {
    pub is_mobile: ReadSignal<bool>,
    pub set_visible: WriteSignal<bool>,
    pub set_mobile_visible: WriteSignal<bool>,
    pub set_active_view: WriteSignal<SidebarView>,
}

impl SidebarControl {
    pub fn show_view(self, view: SidebarView) {
        self.set_active_view.set(view);
        if self.is_mobile.get_untracked() {
            self.set_mobile_visible.set(true);
        } else {
            self.set_visible.set(true);
        }
    }

    pub fn toggle_visible(self) {
        if self.is_mobile.get_untracked() {
            self.set_mobile_visible
                .update(|visible| *visible = !*visible);
        } else {
            self.set_visible.update(|visible| *visible = !*visible);
        }
    }
}

/// Editor content context for outline rendering
#[derive(Clone, Copy)]
pub struct EditorContentContext {
    pub content: ReadSignal<String>,
}

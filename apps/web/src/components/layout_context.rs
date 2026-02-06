// apps/web/src/components/layout_context.rs
//! # 布局相关 Context 类型
//!
//! 定义跨组件共享的布局控制上下文。

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

/// Editor content context for outline rendering
#[derive(Clone, Copy)]
pub struct EditorContentContext {
    pub content: ReadSignal<String>,
}

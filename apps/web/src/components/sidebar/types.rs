// apps\web\src\components\sidebar
//! # Types (类型定义)
//!
//! 包含侧边栏组件之间共享的类型定义，如上下文 Context。

use deve_core::models::DocId;
use leptos::prelude::*;

/// 文件操作上下文
///
/// 用于在文件树组件深层传递操作回调，避免 Prop Drilling。
#[derive(Clone)]
pub struct FileActionsContext {
    pub current_doc: ReadSignal<Option<DocId>>,
    pub on_select: Callback<DocId>,
    pub on_create: Callback<Option<String>>,
    pub on_open_search: Callback<String>,
    pub on_menu_open: Callback<(String, web_sys::MouseEvent)>,
    pub on_menu_close: Callback<()>,
    pub active_menu: ReadSignal<Option<String>>,
    pub on_delete: Callback<String>,
}

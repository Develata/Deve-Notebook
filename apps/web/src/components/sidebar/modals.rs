// apps\web\src\components\sidebar
//! # Context Modals 组件
//!
//! 管理侧边栏所有的弹窗交互逻辑 (创建、重命名、移动)，
//! 从 ExplorerView 中分离以降低代码复杂度。

use crate::components::input_modal::InputModal;
use leptos::prelude::*;

/// 模态框状态枚举
#[derive(Clone, Debug, PartialEq)]
pub enum ModalState {
    None,
    Create { parent: Option<String> },
    Rename { target: String },
    Move { source: String },
}

/// 侧边栏模态框集合组件
#[component]
pub fn SidebarModals(
    modal_state: ReadSignal<ModalState>,
    set_modal_state: WriteSignal<ModalState>,
    #[prop(into)] on_confirm_create: Callback<String>,
    #[prop(into)] on_confirm_rename: Callback<String>,
    #[prop(into)] on_confirm_move: Callback<String>,
) -> impl IntoView {
    view! {
        // 创建文件/文件夹模态框
        <InputModal
            show=Signal::derive(move || matches!(modal_state.get(), ModalState::Create { .. }))
            set_show=Callback::new(move |val: bool| if !val { set_modal_state.set(ModalState::None) })
            title=Signal::derive(move || {
                if let ModalState::Create { parent: Some(p) } = modal_state.get() {
                    format!("Create in '{}'", p)
                } else {
                    "Create New Document".to_string()
                }
            })
            initial_value=Signal::derive(move || None::<String>)
            placeholder="filename or folder/filename"
            confirm_label="Create"
            on_confirm=on_confirm_create
        />

        // 重命名模态框
        <InputModal
            show=Signal::derive(move || matches!(modal_state.get(), ModalState::Rename { .. }))
            set_show=Callback::new(move |val: bool| if !val { set_modal_state.set(ModalState::None) })
            title=Signal::derive(move || "Rename".to_string())
            initial_value=Signal::derive(move || {
                if let ModalState::Rename { target } = modal_state.get() {
                    Some(target.split('/').last().unwrap_or("").to_string())
                } else {
                    None
                }
            })
            placeholder="New name"
            confirm_label="Rename"
            on_confirm=on_confirm_rename
        />

        // 移动模态框
        <InputModal
            show=Signal::derive(move || matches!(modal_state.get(), ModalState::Move { .. }))
            set_show=Callback::new(move |val: bool| if !val { set_modal_state.set(ModalState::None) })
            title=Signal::derive(move || "Move to...".to_string())
            initial_value=Signal::derive(move || {
                if let ModalState::Move { source } = modal_state.get() {
                    Some(source)
                } else {
                    None
                }
            })
            placeholder="New path (e.g. folder/file.md)"
            confirm_label="Move"
            on_confirm=on_confirm_move
        />
    }
}

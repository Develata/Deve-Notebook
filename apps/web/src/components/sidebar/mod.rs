// apps\web\src\components\sidebar
//! # Sidebar Component (侧边栏组件)
//!
//! **架构作用**:
//! 渲染左侧文件导航栏，管理文件操作的交互流程（Context Menu, Modals）。
//!
//! **核心功能清单**:
//! - `Sidebar`: 主组件，接收全局状态与回调。
//! - `build_file_tree`: 将平铺的文档列表构建为树状结构。
//! - File Operations: 处理 Create, Rename, Copy, Move, Delete 的 UI 交互（弹窗确认）。
//! - Clipboard: 管理内部剪贴板状态。
//!
//! **类型**: Core MUST (核心必选)

pub mod components;
pub mod explorer;
pub mod extensions;
pub mod item;
pub mod path_utils;
pub mod repo_switcher;
pub mod source_control;
pub mod tree;
pub mod types;

use self::explorer::ExplorerView;
use self::extensions::ExtensionsView;
use crate::components::activity_bar::SidebarView;
use crate::components::main_layout::SearchControl;

use deve_core::models::DocId;
use leptos::prelude::*;

#[component]
pub fn Sidebar(
    active_view: ReadSignal<SidebarView>,
    docs: ReadSignal<Vec<(DocId, String)>>,
    current_doc: ReadSignal<Option<DocId>>,
    is_readonly: Signal<bool>,
    #[prop(into)] on_select: Callback<DocId>,
    #[prop(into)] on_delete: Callback<String>,
) -> impl IntoView {
    let search_control = expect_context::<SearchControl>();

    Effect::new(move |_| {
        if active_view.get() == SidebarView::Search {
            search_control.set_mode.set("?".to_string());
            search_control.set_show.set(true);
        }
    });

    view! {
        <div class="h-full w-full bg-sidebar border-r border-default">
            {move || match active_view.get() {
                SidebarView::Explorer => view! {
                    <ExplorerView
                        _docs=docs
                        current_doc=current_doc
                        is_readonly=is_readonly
                        on_select=on_select
                        on_delete=on_delete
                    />
                }.into_any(),
                SidebarView::SourceControl => view! {
                    <crate::components::sidebar::source_control::SourceControlView />
                }.into_any(),
                SidebarView::Search => view! {
                    <ExplorerView
                        _docs=docs
                        current_doc=current_doc
                        is_readonly=is_readonly
                        on_select=on_select
                        on_delete=on_delete
                    />
                }.into_any(),
                SidebarView::Extensions => view! {
                    <ExtensionsView />
                }.into_any(),
            }}
        </div>
    }
}

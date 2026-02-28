use crate::components::icons::{ChevronRight, EllipsisVertical, FileText, Plus};
use leptos::prelude::*;

/// 文件/文件夹图标组件
/// 根据节点类型和展开状态显示不同的 SVG 图标
#[component]
pub fn FileIcon(
    #[prop(into)] is_folder: bool,
    #[prop(into)] is_expanded: Signal<bool>,
) -> impl IntoView {
    view! {
        <div class=move || if is_folder { "text-muted" } else { "text-muted opacity-50" }>
             {if is_folder {
                 // 文件夹图标：支持旋转动画
                 view! {
                     <div class="transition-transform duration-200" style=move || if is_expanded.get() { "transform: rotate(90deg)" } else { "" }>
                         <ChevronRight />
                     </div>
                 }.into_any()
             } else {
                 // 文件图标
                 view! {
                     <FileText />
                 }.into_any()
             }}
        </div>
    }
}

/// 树节点操作按钮组
/// 悬停时显示的操作按钮（更多菜单、新建文件等）
#[component]
pub fn ItemActions(
    #[prop(into)] is_folder: bool,
    #[prop(into)] is_menu_open: Signal<bool>,
    #[prop(into)] on_menu: Callback<web_sys::MouseEvent>,
    #[prop(into)] on_create: Callback<web_sys::MouseEvent>,
) -> impl IntoView {
    view! {
        <div
            class="flex items-center gap-1 pr-1 opacity-0 group-hover:opacity-100 transition-opacity duration-200"
            class:opacity-100=move || is_menu_open.get()
        >
            // 更多/菜单 按钮
            <button
                class="p-1 rounded hover:bg-hover text-secondary transition-colors"
                title="More"
                on:click=move |ev| on_menu.run(ev)
            >
                <EllipsisVertical />
            </button>

            {if is_folder {
                view! {
                    // 新建文件按钮 (仅文件夹显示)
                    <button
                        class="p-1 rounded hover:bg-hover text-secondary transition-colors"
                        title="New File"
                        on:click=move |ev| on_create.run(ev)
                    >
                        <Plus />
                    </button>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}

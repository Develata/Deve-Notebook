use leptos::prelude::*;

/// 文件/文件夹图标组件
/// 根据节点类型和展开状态显示不同的 SVG 图标
#[component]
pub fn FileIcon(
    /// 是否为文件夹
    #[prop(into)]
    is_folder: bool,
    /// 文件夹是否展开（仅对文件夹有效）
    #[prop(into)]
    is_expanded: Signal<bool>,
) -> impl IntoView {
    view! {
        <div class=move || if is_folder { "text-gray-400" } else { "text-gray-300" }>
             {if is_folder {
                 // 文件夹图标：支持旋转动画
                 view! {
                     <div class="transition-transform duration-200" style=move || if is_expanded.get() { "transform: rotate(90deg)" } else { "" }>
                         <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                           <path fill-rule="evenodd" d="M7.21 14.77a.75.75 0 01.02-1.06L11.168 10 7.23 6.29a.75.75 0 111.04-1.08l4.5 4.25a.75.75 0 010 1.08l-4.5 4.25a.75.75 0 01-1.06-.02z" clip-rule="evenodd" />
                         </svg>
                     </div>
                 }.into_any()
             } else {
                 // 文件图标
                 view! {
                     <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 opacity-50">
                       <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z" />
                     </svg>
                 }.into_any()
             }}
        </div>
    }
}

/// 树节点操作按钮组
/// 悬停时显示的操作按钮（更多菜单、新建文件等）
#[component]
pub fn ItemActions(
    /// 是否为文件夹
    #[prop(into)]
    is_folder: bool,
    /// 菜单是否打开（保持可见性）
    #[prop(into)]
    is_menu_open: Signal<bool>,
    /// 点击更多菜单的回调
    #[prop(into)]
    on_menu: Callback<web_sys::MouseEvent>,
    /// 点击新建文件的回调
    #[prop(into)]
    on_create: Callback<web_sys::MouseEvent>,
) -> impl IntoView {
    view! {
        <div
            class="flex items-center gap-1 pr-1 opacity-0 group-hover:opacity-100 transition-opacity duration-200"
            class:opacity-100=move || is_menu_open.get()
        >
            // 更多/菜单 按钮
            <button
                class="p-1 rounded hover:bg-gray-300 text-gray-500 transition-colors"
                title="More"
                on:click=move |ev| on_menu.run(ev)
            >
                 <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                   <path d="M10 3a1.5 1.5 0 110 3 1.5 1.5 0 010-3zM10 8.5a1.5 1.5 0 110 3 1.5 1.5 0 010-3zM11.5 15.5a1.5 1.5 0 10-3 0 1.5 1.5 0 003 0z" />
                 </svg>
            </button>

            {if is_folder {
                view! {
                    // 新建文件按钮 (仅文件夹显示)
                    <button
                        class="p-1 rounded hover:bg-gray-300 text-gray-600 transition-colors"
                        title="New File"
                        on:click=move |ev| on_create.run(ev)
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                          <path d="M10.75 4.75a.75.75 0 00-1.5 0v4.5h-4.5a.75.75 0 000 1.5h4.5v4.5a.75.75 0 001.5 0v-4.5h4.5a.75.75 0 000-1.5h-4.5v-4.5z" />
                        </svg>
                    </button>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}

//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::action_visibility::{
    persistent_action_button_class, persistent_action_visibility_class,
};
use crate::components::icons::{ChevronRight, EllipsisVertical, FileText, Plus};
use crate::i18n::{Locale, t};
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
/// 始终可发现的操作按钮（更多菜单、新建文件等）。
#[component]
pub fn ItemActions(
    #[prop(into)] is_folder: bool,
    #[prop(into)] is_readonly: Signal<bool>,
    #[prop(into)] on_menu: Callback<web_sys::MouseEvent>,
    #[prop(into)] on_create: Callback<web_sys::MouseEvent>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    view! {
        <div
            class=format!(
                "flex items-center gap-1 pr-1 {}",
                persistent_action_visibility_class(),
            )
            data-deve-action-visibility="persistent"
        >
            // 更多/菜单 按钮
            <button
                type="button"
                class=persistent_action_button_class()
                title=move || t::sidebar::more(locale.get())
                aria-label=move || t::sidebar::more(locale.get())
                on:click=move |ev| on_menu.run(ev)
            >
                <EllipsisVertical />
            </button>

            {move || if is_folder && !is_readonly.get() {
                view! {
                    // 新建文件按钮 (仅文件夹显示)
                    <button
                        type="button"
                        class=persistent_action_button_class()
                        title=move || t::common::new_file(locale.get())
                        aria-label=move || t::common::new_file(locale.get())
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

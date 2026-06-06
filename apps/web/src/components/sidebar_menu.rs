// apps/web/src/components/sidebar_menu.rs
//! plan_ref:
//!   - 11_ui_design/index#context-action-surface
//!
//! # SidebarMenu 组件 (SidebarMenu Component)
//!
//! 文件树上下文菜单，展示 Context Action 投影并转发 action intent。

use leptos::prelude::*;

use crate::components::dropdown::{Align, AnchorRect, Dropdown};
use crate::context_action::{ContextActionIntent, ContextActionReadiness, ContextActionTarget};
use crate::i18n::Locale;

mod item;

use item::SidebarMenuItems;

#[component]
pub fn SidebarMenu(
    readiness: Signal<ContextActionReadiness>,
    target: ContextActionTarget,
    #[prop(into)] on_action: Callback<ContextActionIntent>,
    #[prop(into)] on_close: Callback<()>,
    anchor: ReadSignal<Option<AnchorRect>>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        <Dropdown anchor=anchor.into() on_close=on_close align=Align::Right offset=6.0>
            <div class="w-48 bg-panel rounded-md shadow-lg border border-default py-1 text-sm text-primary select-none animate-in fade-in zoom-in-95 duration-100 ease-out origin-top-right">
                <SidebarMenuItems locale readiness target on_action on_close />
            </div>
        </Dropdown>
    }
}

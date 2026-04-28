// apps/web/src/components/activity_bar/popup_menu.rs
//! plan_ref:
//!   - 08_ui_design_01_web#web-layout-persistence
//!
//! # ActivityBar Popup Menu
//!
//! 弹出菜单，用于切换/固定侧边栏视图。

use super::types::SidebarView;
use crate::components::icons::Pin;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn ViewPopupMenu(
    active_view: ReadSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    select_view: Callback<SidebarView>,
    toggle_pin: Callback<SidebarView>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    SidebarView::all()
        .into_iter()
        .map(|item| {
            let is_pinned = move || pinned_views.get().contains(&item);
            let is_active = move || active_view.get() == item;
            view! {
                <div class="px-2 py-1 flex items-center gap-2 text-sm text-primary">
                    <button
                        class="flex-1 rounded-md px-2 py-1.5 text-left hover:bg-hover"
                        on:click=move |_| select_view.run(item)
                    >
                        <span class=move || if is_active() { "font-bold" } else { "" }>
                            {move || item.title(locale.get())}
                        </span>
                    </button>
                    {move || if is_pinned() {
                        view! {
                            <button
                                class="rounded-md p-1.5 text-accent hover:bg-hover"
                                title=move || t::common::unpin(locale.get())
                                on:click=move |_| toggle_pin.run(item)
                            >
                                <Pin class="w-3.5 h-3.5"/>
                            </button>
                        }.into_any()
                    } else {
                        view! {
                            <button
                                class="rounded-md p-1.5 text-muted hover:bg-hover hover:text-primary"
                                title=move || t::common::pin(locale.get())
                                on:click=move |_| toggle_pin.run(item)
                            >
                                <Pin class="w-3.5 h-3.5"/>
                            </button>
                        }.into_any()
                    }}
                </div>
            }
        })
        .collect::<Vec<_>>()
}

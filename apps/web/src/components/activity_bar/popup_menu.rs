// apps/web/src/components/activity_bar/popup_menu.rs
//! # ActivityBar Popup Menu
//!
//! 弹出菜单，用于切换/固定侧边栏视图。

use super::types::SidebarView;
use crate::components::icons::Pin;
use crate::i18n::Locale;
use leptos::prelude::*;

#[component]
pub fn ViewPopupMenu(
    active_view: ReadSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    toggle_pin: Callback<SidebarView>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    SidebarView::all()
        .into_iter()
        .map(|item| {
            let is_pinned = move || pinned_views.get().contains(&item);
            let is_active = move || active_view.get() == item;
            view! {
                <div
                    class="px-3 py-2 hover:bg-hover cursor-pointer flex items-center justify-between text-sm text-primary"
                    on:click=move |_| toggle_pin.run(item)
                >
                    <span class=move || if is_active() { "font-bold" } else { "" }>
                        {item.title(locale.get())}
                    </span>
                    {move || if is_pinned() {
                        view! {
                            <Pin class="w-3.5 h-3.5 text-accent"/>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                </div>
            }
        })
        .collect::<Vec<_>>()
}

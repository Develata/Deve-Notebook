// apps/web/src/components/activity_bar/popup_menu.rs
//! # ActivityBar Popup Menu
//!
//! 弹出菜单，用于切换/固定侧边栏视图。

use super::types::SidebarView;
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
                            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-accent"><line x1="12" y1="17" x2="12" y2="22"></line><path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"></path></svg>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                </div>
            }
        })
        .collect::<Vec<_>>()
}

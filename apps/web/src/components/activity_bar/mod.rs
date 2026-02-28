// apps/web/src/components/activity_bar/mod.rs
//! # ActivityBar 组件 (ActivityBar Component)
//!
//! 侧边栏导航条，用于在不同的视图（资源管理器、搜索、源码管理、扩展）之间切换。

mod popup_menu;
mod types;

pub use types::SidebarView;

use crate::i18n::Locale;
use leptos::prelude::*;
use popup_menu::ViewPopupMenu;

#[component]
pub fn ActivityBar(
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
) -> impl IntoView {
    let (show_more, set_show_more) = signal(false);
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    let backdrop = move || {
        if show_more.get() {
            view! {
                <div class="fixed inset-0 z-30" on:click=move |_| set_show_more.set(false)></div>
            }
            .into_any()
        } else {
            view! {}.into_any()
        }
    };

    let icon_btn = move |view: SidebarView| {
        let is_active = move || active_view.get() == view;
        let icon = view.icon();
        view! {
            <button
                class=move || format!(
                    "p-2 mr-1 rounded-lg transition-colors relative group {}",
                    if is_active() { "text-accent" } else { "text-muted hover:text-primary" }
                )
                title=move || view.title(locale.get())
                on:click=move |_| set_active_view.set(view)
            >
                <div class="w-4 h-4" inner_html=icon></div>
                {move || if is_active() {
                    view! { <div class="absolute bottom-0 left-2 right-2 h-0.5 bg-accent rounded-t"></div> }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </button>
        }
    };

    let toggle_pin = Callback::new(move |view: SidebarView| {
        set_pinned_views.update(|pinned| {
            if pinned.contains(&view) {
                pinned.retain(|&v| v != view);
            } else {
                pinned.push(view);
            }
        });
    });

    view! {
        {backdrop}
        <div class="w-full h-10 flex flex-row items-center px-1 bg-sidebar border-b border-default flex-none overflow-visible">
            <div class="flex-1 flex flex-row items-center overflow-x-auto no-scrollbar mask-gradient">
                <For
                    each=move || pinned_views.get()
                    key=|view| *view
                    children=move |view| view! { {icon_btn(view)} }
                />
            </div>

            <div class="flex-none flex items-center relative ml-1">
                <button
                    class="p-2 text-muted hover:text-primary rounded-lg transition-colors"
                    title="More..."
                    on:click=move |_| set_show_more.update(|v| *v = !*v)
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/></svg>
                </button>

                {move || if show_more.get() {
                    view! {
                        <div class="absolute right-0 top-full mt-1 w-48 bg-panel shadow-xl rounded-lg border border-default py-1 z-50">
                            <ViewPopupMenu
                                active_view=active_view
                                pinned_views=pinned_views
                                toggle_pin=toggle_pin
                                locale=locale
                            />
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
        </div>
    }
}

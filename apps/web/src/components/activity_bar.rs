// apps\web\src\components
//! # ActivityBar 组件 (ActivityBar Component)
//!
//! 侧边栏导航条，用于在不同的视图（资源管理器、搜索、源码管理、扩展）之间切换。

use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum SidebarView {
    #[default]
    Explorer, // 资源管理器
    Search,        // 搜索
    SourceControl, // 源代码管理 (Git)
    Extensions,    // 扩展
}

impl SidebarView {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Explorer,
            Self::Search,
            Self::SourceControl,
            Self::Extensions,
        ]
    }

    pub fn title(&self, locale: Locale) -> &'static str {
        match self {
            Self::Explorer => t::sidebar::explorer(locale),
            Self::Search => t::sidebar::search(locale),
            Self::SourceControl => t::sidebar::source_control(locale),
            Self::Extensions => t::sidebar::extensions(locale),
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Explorer => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>"#
            }
            Self::Search => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>"#
            }
            Self::SourceControl => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/><path d="M6 9v12"/><path d="M18 9v12"/><path d="M12 15V3"/></svg>"#
            }
            Self::Extensions => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="9" height="9" rx="2"/><rect x="13" y="2" width="9" height="9" rx="2"/><rect x="13" y="13" width="9" height="9" rx="2"/><line x1="8" y1="21" x2="8" y2="12"/><line x1="8" y1="12" x2="3" y2="12"/><path d="M2.5 21h5.5a2 2 0 0 0 2-2v-5a2 2 0 0 0-2-2H2.5a.5.5 0 0 0-.5.5v8a.5.5 0 0 0 .5.5z"/></svg>"#
            }
        }
    }
}

#[component]
pub fn ActivityBar(
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
) -> impl IntoView {
    let (show_more, set_show_more) = signal(false);
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    // Close menu when clicking outside (simple version)
    // For a robust implementation, use leptos_use::onClickOutside, but here we'll use a backdrop.
    let backdrop = move || {
        if show_more.get() {
            view! {
                <div
                    class="fixed inset-0 z-30"
                    on:click=move |_| set_show_more.set(false)
                ></div>
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
                    if is_active() { "text-blue-600 dark:text-blue-400" } else { "text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-100" }
                )
                title=move || view.title(locale.get())
                on:click=move |_| set_active_view.set(view)
            >
                <div class="w-4 h-4" inner_html=icon></div>

                // 活动指示器 (Horizontal)
                {move || if is_active() {
                    view! { <div class="absolute bottom-0 left-2 right-2 h-0.5 bg-blue-400 rounded-t"></div> }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </button>
        }
    };

    let toggle_pin = move |view: SidebarView| {
        set_pinned_views.update(|pinned| {
            if pinned.contains(&view) {
                pinned.retain(|&v| v != view);
            } else {
                pinned.push(view);
            }
        });
    };

    view! {
        {backdrop}
        <div class="w-full h-10 flex flex-row items-center px-1 bg-[#f3f3f3] dark:bg-[#252526] border-b border-[#e5e5e5] dark:border-[#1e1e1e] flex-none overflow-visible">
            // Render pinned views (Scrollable)
            <div class="flex-1 flex flex-row items-center overflow-x-auto no-scrollbar mask-gradient">
                <For
                    each=move || pinned_views.get()
                    key=|view| *view
                    children=move |view| {
                        view! {
                             {icon_btn(view)}
                        }
                    }
                />
            </div>

            // More / Config Button (Static, allow popup overflow)
            <div class="flex-none flex items-center relative ml-1">
                <button
                    class="p-2 text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-100 rounded-lg transition-colors"
                    title="More..."
                    on:click=move |_| set_show_more.update(|v| *v = !*v)
                >
                     // Ellipsis Horizontal Icon
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/></svg>
                </button>

                // Popup Menu
                {move || if show_more.get() {
                    view! {
                        <div class="absolute right-0 top-full mt-1 w-48 bg-white dark:bg-[#252526] shadow-xl rounded-lg border border-gray-200 dark:border-[#1e1e1e] py-1 z-50">
                             {
                                SidebarView::all().into_iter().map(|item| {
                                    let is_pinned = move || pinned_views.get().contains(&item);
                                    let is_active = move || active_view.get() == item;
                                    view! {
                                        <div
                                            class="px-3 py-2 hover:bg-gray-100 dark:hover:bg-[#37373d] cursor-pointer flex items-center justify-between text-sm text-gray-700 dark:text-gray-200"
                                            on:click=move |_| toggle_pin(item)
                                        >
                                            <span class=move || if is_active() { "font-bold" } else { "" }>
                                                {item.title(locale.get())}
                                            </span>

                                            // Pin Icon if pinned
                                            {move || if is_pinned() {
                                                view! {
                                                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-blue-500"><line x1="12" y1="17" x2="12" y2="22"></line><path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"></path></svg>
                                                }.into_any()
                                            } else {
                                                view! {}.into_any()
                                            }}
                                        </div>
                                    }
                                }).collect::<Vec<_>>()
                             }
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
        </div>
    }
}

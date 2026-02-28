// apps/web/src/components/mobile_layout/drawers/left.rs

use crate::components::activity_bar::SidebarView;
use crate::components::sidebar::Sidebar;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::html;
use leptos::prelude::*;

use super::drawer_class;

#[component]
pub fn LeftDrawer(
    core: CoreState,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    open: ReadSignal<bool>,
    on_doc_select: Callback<deve_core::models::DocId>,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (show_more, set_show_more) = signal(false);
    let more_menu_ref = NodeRef::<html::Div>::new();

    let title = Signal::derive(move || match active_view.get() {
        SidebarView::Explorer => t::sidebar::explorer(locale.get()),
        SidebarView::Search => t::sidebar::search(locale.get()),
        SidebarView::SourceControl => t::sidebar::source_control(locale.get()),
        SidebarView::Extensions => t::sidebar::extensions(locale.get()),
    });

    let toggle_pin = move |view: SidebarView| {
        set_pinned_views.update(|pinned| {
            if pinned.contains(&view) {
                if pinned.len() > 1 {
                    pinned.retain(|v| *v != view);
                }
            } else {
                pinned.push(view);
            }
        });
    };

    let tab = move |view: SidebarView, label: Signal<&'static str>| {
        view! {
            <button
                class=move || {
                    let state = if active_view.get() == view {
                        "bg-accent-subtle border border-b-accent text-accent"
                    } else {
                        "bg-panel border border-default text-secondary active:bg-hover"
                    };
                    format!(
                        "mobile-sidebar-tab {} h-11 min-w-12 px-3 rounded-md active:scale-95 transition-transform duration-150 ease-out {}",
                        sidebar_tab_class(view),
                        state
                    )
                }
                on:click=move |_| {
                    set_active_view.set(view);
                    set_show_more.set(false);
                }
                title=move || label.get().to_string()
                aria-label=move || label.get().to_string()
            >
                <div class="w-4 h-4 mx-auto" inner_html=view.icon()></div>
            </button>
        }
    };

    Effect::new(move |_| {
        if !open.get() {
            set_show_more.set(false);
        }
    });

    Effect::new(move |_| {
        if show_more.get()
            && let Some(el) = more_menu_ref.get()
        {
            let _ = el.focus();
        }
    });

    view! {
        <div class=move || drawer_class("left", open.get())>
            <div class="flex flex-col h-full">
                <div
                    class="h-12 px-3 flex items-center justify-between border-b border-default text-sm font-semibold"
                    style="padding-top: env(safe-area-inset-top);"
                >
                    <span class="text-primary flex items-center gap-1">{move || title.get().to_string()}</span>
                    <button
                        class="h-11 min-w-11 px-3 text-sm font-medium text-secondary rounded-md hover:bg-hover active:bg-active transition-colors duration-200 ease-out"
                        title=move || t::sidebar::close_file_tree(locale.get())
                        aria-label=move || t::sidebar::close_file_tree(locale.get())
                        on:click=move |_| on_close.run(())
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4 mx-auto">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M6 6l8 8M14 6l-8 8" />
                        </svg>
                    </button>
                </div>

                <div class="px-2 py-2 border-b border-default relative">
                    <div class="flex items-center gap-2 w-full">
                        <div class="flex-1 overflow-x-auto">
                            <div class="flex items-center gap-2 min-w-max">
                                <For
                                    each=move || pinned_views.get()
                                    key=|v| *v
                                    children=move |view| {
                                        let label = Signal::derive(move || match view {
                                            SidebarView::Explorer => t::sidebar::explorer(locale.get()),
                                            SidebarView::Search => t::sidebar::search(locale.get()),
                                            SidebarView::SourceControl => t::sidebar::source_control(locale.get()),
                                            SidebarView::Extensions => t::sidebar::extensions(locale.get()),
                                        });
                                        tab(view, label)
                                    }
                                />
                            </div>
                        </div>
                        <button
                            class="mobile-more-button h-11 min-w-11 px-2 rounded-md bg-panel border border-default text-secondary active:bg-hover active:scale-95 transition-transform duration-150 ease-out"
                            title=move || t::sidebar::more(locale.get())
                            aria-label=move || t::sidebar::more(locale.get())
                            on:click=move |_| set_show_more.update(|v| *v = !*v)
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mx-auto"><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/></svg>
                        </button>
                    </div>

                    {move || if show_more.get() {
                        view! {
                            <div class="mobile-more-backdrop fixed inset-0 z-[51]" on:click=move |_| set_show_more.set(false)></div>
                            <div
                                class="mobile-more-panel absolute right-2 top-full mt-1 w-44 bg-panel shadow-xl rounded-lg border border-default py-1 z-[52]"
                                node_ref=more_menu_ref
                                tabindex="-1"
                                role="menu"
                                on:keydown=move |ev| {
                                    if ev.key() == "Escape" {
                                        ev.prevent_default();
                                        set_show_more.set(false);
                                    }
                                }
                            >
                                {SidebarView::all().into_iter().map(|item| {
                                    let pinned = Signal::derive(move || pinned_views.get().contains(&item));
                                    view! {
                                        <button
                                            class=format!(
                                                "mobile-more-item {} w-full h-11 px-3 text-left text-sm text-primary active:bg-hover flex items-center justify-between",
                                                more_item_class(item)
                                            )
                                            role="menuitem"
                                            on:click=move |_| {
                                                toggle_pin(item);
                                                set_show_more.set(false);
                                            }
                                        >
                                            <span class=move || if active_view.get() == item { "font-semibold" } else { "" }>{item.title(locale.get())}</span>
                                            <span class=move || if pinned.get() { "text-accent" } else { "text-transparent" }>
                                                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="17" x2="12" y2="22"></line><path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"></path></svg>
                                            </span>
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                </div>

                <div class="flex-1 overflow-hidden px-2 pb-3" style="padding-bottom: env(safe-area-inset-bottom);">
                    <div class="h-full overflow-y-auto">
                        <Sidebar
                            active_view=active_view
                            docs=core.docs
                            current_doc=core.current_doc
                            on_select=Callback::new(move |id| {
                                on_doc_select.run(id);
                                on_close.run(())
                            })
                            on_delete=core.on_doc_delete
                        />
                    </div>
                </div>
            </div>
        </div>
    }
}

fn sidebar_tab_class(view: SidebarView) -> &'static str {
    match view {
        SidebarView::Explorer => "mobile-tab-explorer",
        SidebarView::Search => "mobile-tab-search",
        SidebarView::SourceControl => "mobile-tab-source-control",
        SidebarView::Extensions => "mobile-tab-extensions",
    }
}

fn more_item_class(view: SidebarView) -> &'static str {
    match view {
        SidebarView::Explorer => "more_menu_item_explorer",
        SidebarView::Search => "more_menu_item_search",
        SidebarView::SourceControl => "more_menu_item_source_control",
        SidebarView::Extensions => "more_menu_item_extensions",
    }
}

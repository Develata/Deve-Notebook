//! plan_ref:
//!   - 11_ui_design_03_mobile#mobile-interaction-design
//!   - 11_ui_design_03_mobile#mobile-responsive-layout
//!
use crate::components::activity_bar::SidebarView;
use crate::components::icons::Pin;
use crate::i18n::{Locale, t};
use leptos::html;
use leptos::prelude::*;

pub(super) fn mobile_more_menu_marker(open: bool) -> Option<&'static str> {
    open.then_some("visible")
}

pub(super) fn mobile_more_item_marker(view: SidebarView) -> &'static str {
    match view {
        SidebarView::Explorer => "more_menu_item_explorer",
        SidebarView::Search => "more_menu_item_search",
        SidebarView::SourceControl => "more_menu_item_source_control",
        SidebarView::Extensions => "more_menu_item_extensions",
    }
}

pub(super) fn mobile_more_pin_action_marker(view: SidebarView) -> &'static str {
    match view {
        SidebarView::Explorer => "more_menu_pin_explorer",
        SidebarView::Search => "more_menu_pin_search",
        SidebarView::SourceControl => "more_menu_pin_source_control",
        SidebarView::Extensions => "more_menu_pin_extensions",
    }
}

pub(super) fn mobile_more_after_item_click() -> bool {
    false
}

pub(super) fn mobile_more_after_escape() -> bool {
    false
}

pub(super) fn mobile_more_after_pin_click(open: bool) -> bool {
    open
}

pub(super) fn toggle_mobile_more_pin(pinned: &mut Vec<SidebarView>, view: SidebarView) -> bool {
    if pinned.contains(&view) {
        if pinned.len() <= 1 {
            return false;
        }
        pinned.retain(|v| *v != view);
        return true;
    }

    pinned.push(view);
    true
}

#[component]
pub(super) fn LeftDrawerMoreMenu(
    locale: RwSignal<Locale>,
    active_view: ReadSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    select_view: Callback<SidebarView>,
    show_more: ReadSignal<bool>,
    set_show_more: WriteSignal<bool>,
    more_menu_ref: NodeRef<html::Div>,
) -> impl IntoView {
    let toggle_pin = move |view: SidebarView| {
        set_pinned_views.update(|pinned| {
            let _ = toggle_mobile_more_pin(pinned, view);
        });
    };

    view! {
        {move || if show_more.get() {
            view! {
                <div class="mobile-more-backdrop fixed inset-0 z-[var(--z-floating)]" on:click=move |_| set_show_more.set(false)></div>
                <div
                    class="mobile-more-panel absolute right-2 top-full mt-1 w-44 bg-panel shadow-xl rounded-lg border border-default py-1 z-[calc(var(--z-floating)_+_1)]"
                    data-deve-mobile-more-menu=move || mobile_more_menu_marker(show_more.get())
                    node_ref=more_menu_ref
                    tabindex="-1"
                    role="menu"
                    on:keydown=move |ev| {
                        if ev.key() == "Escape" {
                            ev.prevent_default();
                            set_show_more.set(mobile_more_after_escape());
                        }
                    }
                >
                    {SidebarView::all().into_iter().map(|item| {
                        let pinned = Signal::derive(move || pinned_views.get().contains(&item));
                        view! {
                            <div
                                class=format!(
                                    "mobile-more-item {} w-full h-11 px-3 text-left text-sm text-primary active:bg-hover flex items-center justify-between",
                                    more_item_class(item)
                                )
                            >
                                <button
                                    data-deve-mobile-more-item=mobile_more_item_marker(item)
                                    class="flex-1 h-full text-left"
                                    role="menuitem"
                                    on:click=move |_| {
                                        select_view.run(item);
                                        set_show_more.set(mobile_more_after_item_click());
                                    }
                                >
                                    <span class=move || if active_view.get() == item { "font-semibold" } else { "" }>{item.title(locale.get())}</span>
                                </button>
                                <button
                                    data-deve-mobile-more-pin-action=mobile_more_pin_action_marker(item)
                                    class=move || format!(
                                        "rounded-md p-1.5 {}",
                                        if pinned.get() {
                                            "text-accent active:bg-hover"
                                        } else {
                                            "text-muted active:bg-hover"
                                        }
                                    )
                                    title=move || {
                                        if pinned.get() {
                                            t::common::unpin(locale.get())
                                        } else {
                                            t::common::pin(locale.get())
                                        }
                                    }
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        toggle_pin(item);
                                        set_show_more.set(mobile_more_after_pin_click(show_more.get()));
                                    }
                                >
                                    <Pin class="w-3.5 h-3.5"/>
                                </button>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            }.into_any()
        } else {
            view! {}.into_any()
        }}
    }
}

fn more_item_class(view: SidebarView) -> &'static str {
    mobile_more_item_marker(view)
}

#[cfg(test)]
mod tests;

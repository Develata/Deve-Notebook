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

pub(super) fn activity_more_item_marker(view: SidebarView) -> &'static str {
    match view {
        SidebarView::Explorer => "activity_more_item_explorer",
        SidebarView::Search => "activity_more_item_search",
        SidebarView::SourceControl => "activity_more_item_source_control",
        SidebarView::Extensions => "activity_more_item_extensions",
    }
}

pub(super) fn activity_more_pin_action_marker(view: SidebarView) -> &'static str {
    match view {
        SidebarView::Explorer => "activity_more_pin_explorer",
        SidebarView::Search => "activity_more_pin_search",
        SidebarView::SourceControl => "activity_more_pin_source_control",
        SidebarView::Extensions => "activity_more_pin_extensions",
    }
}

pub(super) fn activity_more_after_item_click() -> bool {
    false
}

pub(super) fn activity_more_after_pin_click(open: bool) -> bool {
    open
}

pub(super) fn toggle_activity_more_pin(pinned: &mut Vec<SidebarView>, view: SidebarView) -> bool {
    if pinned.contains(&view) {
        pinned.retain(|v| *v != view);
        return true;
    }

    pinned.push(view);
    true
}

#[component]
pub fn ViewPopupMenu(
    active_view: ReadSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    select_view: Callback<SidebarView>,
    toggle_pin: Callback<SidebarView>,
    show_more: ReadSignal<bool>,
    set_show_more: WriteSignal<bool>,
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
                        data-deve-activity-more-item=activity_more_item_marker(item)
                        class="flex-1 rounded-md px-2 py-1.5 text-left hover:bg-hover"
                        role="menuitem"
                        on:click=move |_| select_view.run(item)
                    >
                        <span class=move || if is_active() { "font-bold" } else { "" }>
                            {move || item.title(locale.get())}
                        </span>
                    </button>
                    {move || if is_pinned() {
                        view! {
                            <button
                                data-deve-activity-more-pin-action=activity_more_pin_action_marker(item)
                                class="rounded-md p-1.5 text-accent hover:bg-hover"
                                title=move || t::common::unpin(locale.get())
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    toggle_pin.run(item);
                                    set_show_more.set(activity_more_after_pin_click(show_more.get()));
                                }
                            >
                                <Pin class="w-3.5 h-3.5"/>
                            </button>
                        }.into_any()
                    } else {
                        view! {
                            <button
                                data-deve-activity-more-pin-action=activity_more_pin_action_marker(item)
                                class="rounded-md p-1.5 text-muted hover:bg-hover hover:text-primary"
                                title=move || t::common::pin(locale.get())
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    toggle_pin.run(item);
                                    set_show_more.set(activity_more_after_pin_click(show_more.get()));
                                }
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

#[cfg(test)]
mod tests;

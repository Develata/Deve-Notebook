// apps/web/src/components/mobile_layout/drawers/mod.rs
//! # Mobile Drawers

mod left;
mod right;

use crate::components::activity_bar::SidebarView;
use crate::hooks::use_core::CoreState;
use leptos::prelude::*;

use left::LeftDrawer;
use right::RightDrawer;

#[component]
pub fn MobileDrawers(
    core: CoreState,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    show_sidebar: ReadSignal<bool>,
    show_outline: ReadSignal<bool>,
    on_doc_select: Callback<deve_core::models::DocId>,
    on_close: Callback<()>,
    content_signal: Option<ReadSignal<String>>,
) -> impl IntoView {
    view! {
        <LeftDrawer
            core=core.clone()
            active_view=active_view
            set_active_view=set_active_view
            pinned_views=pinned_views
            set_pinned_views=set_pinned_views
            open=show_sidebar
            on_doc_select=on_doc_select
            on_close=on_close
        />

        <RightDrawer
            open=show_outline
            on_close=on_close
            content_signal=content_signal
        />
    }
}

pub(super) fn drawer_class(side: &str, open: bool) -> String {
    let base = if side == "left" {
        "fixed inset-y-0 left-0"
    } else {
        "fixed inset-y-0 right-0"
    };
    let width = "w-[78%] max-w-[320px]";
    let surface = "bg-panel z-50 shadow-lg transform transition-transform duration-200 ease-out";
    let offset = if open {
        "translate-x-0"
    } else if side == "left" {
        "-translate-x-full"
    } else {
        "translate-x-full"
    };
    format!("{} {} {} {}", base, width, surface, offset)
}

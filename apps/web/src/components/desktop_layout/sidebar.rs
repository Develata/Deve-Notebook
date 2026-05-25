//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::activity_bar::{ActivityBar, SidebarView};
use crate::components::sidebar::Sidebar;
use crate::hooks::use_core::CoreState;
use leptos::prelude::*;

#[component]
pub fn DesktopSidebar(
    core: CoreState,
    sidebar_width: ReadSignal<i32>,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
) -> impl IntoView {
    view! {
        <aside
            data-deve-desktop-col="1-sidebar"
            class="flex-none bg-panel rounded-lg shadow-sm border border-default flex flex-col z-[var(--z-panels)]"
            style=move || format!("width: {}px", sidebar_width.get())
        >
            <ActivityBar
                active_view=active_view
                set_active_view=set_active_view
                pinned_views=pinned_views
                set_pinned_views=set_pinned_views
            />
            <div class="flex-1 overflow-hidden">
                <Sidebar
                    active_view=active_view
                    docs=core.docs
                    current_doc=core.current_doc
                    is_readonly=core.is_spectator
                    on_select=core.on_doc_select
                    on_delete=core.on_doc_delete
                />
            </div>
        </aside>
    }
}

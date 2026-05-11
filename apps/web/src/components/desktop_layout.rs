// apps/web/src/components/desktop_layout.rs
//! plan_ref:
//!   - 08_ui_design_01_web#web-layout-persistence
//!
//! # Desktop Layout

use self::banner::DesktopSyncBanner;
use self::content::DesktopLayoutContent;
use self::handles::{DesktopInnerResizeHandle, DesktopOuterResizeHandle};
use self::sidebar::DesktopSidebar;
use crate::components::activity_bar::SidebarView;
use crate::components::desktop_chat_panel::DesktopChatPanel;
use crate::components::header::Header;
use crate::hooks::use_core::CoreState;
use crate::hooks::use_layout::LayoutHookReturn;
use leptos::prelude::*;

mod banner;
mod content;
mod handles;
mod sidebar;

#[component]
pub fn DesktopLayout(
    core: CoreState,
    layout: LayoutHookReturn,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
    on_logout: Callback<()>,
    chat_visible: ReadSignal<bool>,
) -> impl IntoView {
    let (
        sidebar_width,
        right_width,
        outer_gutter,
        start_resize_left,
        start_resize_right,
        start_resize_outer_left,
        start_resize_outer_right,
        _stop_resize,
        _do_resize,
        _is_resizing,
    ) = layout;

    view! {
        <Header
            status_text=core.status_text
            on_home=on_home
            on_open=on_open
            on_command=on_command
            on_logout=on_logout
        />
        <DesktopSyncBanner sync_banner=core.sync_banner />
        <main
            class="flex-1 w-full flex overflow-hidden relative"
            style=move || {
                format!(
                    "padding-left: {}px; padding-right: {}px;",
                    outer_gutter.get(),
                    outer_gutter.get()
                )
            }
        >
            <DesktopOuterResizeHandle
                side="left"
                outer_gutter=outer_gutter
                on_resize=start_resize_outer_left
            />
            <DesktopOuterResizeHandle
                side="right"
                outer_gutter=outer_gutter
                on_resize=start_resize_outer_right
            />

            <DesktopSidebar
                core=core.clone()
                sidebar_width=sidebar_width
                active_view=active_view
                set_active_view=set_active_view
                pinned_views=pinned_views
                set_pinned_views=set_pinned_views
            />

            <DesktopInnerResizeHandle on_resize=start_resize_left />

            <DesktopLayoutContent core=core.clone() />

            <DesktopChatPanel
                chat_visible=chat_visible
                right_width=right_width
                start_resize_right=start_resize_right
            />
        </main>
    }
}

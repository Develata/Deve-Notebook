// apps/web/src/components/desktop_layout.rs
//! # Desktop Layout

use self::desktop_layout_banner::DesktopSyncBanner;
use self::desktop_layout_content::DesktopLayoutContent;
use self::desktop_layout_sidebar::DesktopSidebar;
use crate::components::activity_bar::SidebarView;
use crate::components::desktop_chat_panel::DesktopChatPanel;
use crate::components::header::Header;
use crate::hooks::use_core::CoreState;
use crate::hooks::use_layout::LayoutHookReturn;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[path = "desktop_layout_banner.rs"]
mod desktop_layout_banner;
#[path = "desktop_layout_content.rs"]
mod desktop_layout_content;
#[path = "desktop_layout_sidebar.rs"]
mod desktop_layout_sidebar;

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
            <div
                class="absolute top-0 h-full w-3 cursor-col-resize touch-none"
                style=move || format!("left: {}px; transform: translateX(-50%);", outer_gutter.get())
                on:pointerdown=move |ev| {
                    if let Some(target) = ev.target()
                        && let Ok(el) = target.dyn_into::<web_sys::Element>()
                    {
                        let _ = el.set_pointer_capture(ev.pointer_id());
                    }
                    start_resize_outer_left.run(ev)
                }
            ></div>
            <div
                class="absolute top-0 h-full w-3 cursor-col-resize touch-none"
                style=move || format!("right: {}px; transform: translateX(50%);", outer_gutter.get())
                on:pointerdown=move |ev| {
                    if let Some(target) = ev.target()
                        && let Ok(el) = target.dyn_into::<web_sys::Element>()
                    {
                        let _ = el.set_pointer_capture(ev.pointer_id());
                    }
                    start_resize_outer_right.run(ev)
                }
            ></div>

            <DesktopSidebar
                core=core.clone()
                sidebar_width=sidebar_width
                active_view=active_view
                set_active_view=set_active_view
                pinned_views=pinned_views
                set_pinned_views=set_pinned_views
            />

            <div
                class="w-4 flex-none cursor-col-resize flex items-center justify-center hover:bg-accent-subtle group transition-colors touch-none"
                on:pointerdown=move |ev| {
                    if let Some(target) = ev.target()
                        && let Ok(el) = target.dyn_into::<web_sys::Element>()
                    {
                        let _ = el.set_pointer_capture(ev.pointer_id());
                    }
                    start_resize_left.run(ev)
                }
            >
                <div class="w-[1px] h-8 bg-active group-hover:bg-accent transition-colors"></div>
            </div>

            <DesktopLayoutContent core=core.clone() />

            <DesktopChatPanel
                chat_visible=chat_visible
                right_width=right_width
                start_resize_right=start_resize_right
            />
        </main>
    }
}

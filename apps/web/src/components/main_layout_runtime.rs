//! plan_ref:
//!   - 08_ui_design_01_web#web-layout-persistence
//!   - 06_repository#repo-scope-runtime
//!
use crate::components::activity_bar::SidebarView;
use crate::components::disconnect_overlay::DisconnectedOverlay;
use crate::hooks::use_core::CoreState;
use crate::hooks::use_layout::LayoutHookReturn;
use leptos::prelude::*;

mod body;
mod overlays;

use self::body::MainLayoutBody;
use self::overlays::MainLayoutOverlays;

#[component]
pub fn MainLayoutRuntime(
    core: CoreState,
    desktop_layout: LayoutHookReturn,
    is_mobile: ReadSignal<bool>,
    is_resizing: ReadSignal<bool>,
    do_resize: Callback<web_sys::PointerEvent>,
    stop_resize: Callback<()>,
    show_search: ReadSignal<bool>,
    set_show_search: WriteSignal<bool>,
    search_mode: ReadSignal<String>,
    show_settings: ReadSignal<bool>,
    set_show_settings: WriteSignal<bool>,
    active_view: ReadSignal<SidebarView>,
    set_active_view: WriteSignal<SidebarView>,
    pinned_views: ReadSignal<Vec<SidebarView>>,
    set_pinned_views: WriteSignal<Vec<SidebarView>>,
    chat_visible: ReadSignal<bool>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
    on_settings: Callback<()>,
    on_logout: Callback<()>,
) -> impl IntoView {
    view! {
        <div
            class="h-screen w-screen flex flex-col bg-sidebar text-primary font-sans"
            on:pointermove=move |ev| do_resize.run(ev)
            on:pointerup=move |_| stop_resize.run(())
            on:pointerleave=move |_| stop_resize.run(())
            on:pointercancel=move |_| stop_resize.run(())
            tabindex="-1"
            style=move || if is_resizing.get() { "cursor: col-resize; user-select: none;" } else { "" }
        >
            <MainLayoutOverlays
                core=core.clone()
                is_mobile=is_mobile
                show_search=show_search
                set_show_search=set_show_search
                search_mode=search_mode
                show_settings=show_settings
                set_show_settings=set_show_settings
                on_settings=on_settings
                on_open=on_open
            />

            <MainLayoutBody
                core=core.clone()
                desktop_layout=desktop_layout
                is_mobile=is_mobile
                active_view=active_view
                set_active_view=set_active_view
                pinned_views=pinned_views
                set_pinned_views=set_pinned_views
                chat_visible=chat_visible
                on_home=on_home
                on_open=on_open
                on_command=on_command
                on_logout=on_logout
            />
            <DisconnectedOverlay status=core.ws.status.into() />
        </div>
    }
}

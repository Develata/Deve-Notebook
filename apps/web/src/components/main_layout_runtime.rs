use crate::components::activity_bar::SidebarView;
use crate::components::desktop_layout::DesktopLayout;
use crate::components::disconnect_overlay::DisconnectedOverlay;
use crate::components::merge_modal_slot::MergeModalSlot;
use crate::components::mobile_layout::MobileLayout;
use crate::components::pending_navigation_modal::PendingNavigationModal;
use crate::hooks::use_core::CoreState;
use crate::hooks::use_layout::LayoutHookReturn;
use leptos::prelude::*;

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
) -> impl IntoView {
    let core_for_layout = core.clone();
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
            <crate::components::search_box::UnifiedSearch
                show=show_search
                set_show=set_show_search
                mode_signal=Signal::derive(move || search_mode.get())
                ui_mode=Signal::derive(move || {
                    if is_mobile.get() {
                        crate::components::search_box::SearchUiMode::Sheet
                    } else {
                        crate::components::search_box::SearchUiMode::Overlay
                    }
                })
                on_settings=on_settings
                on_open=on_open
            />

            <crate::components::settings::SettingsModal
                show=show_settings
                set_show=set_show_settings
            />

            <MergeModalSlot />
            <PendingNavigationModal
                pending=core.pending_navigation
                set_pending=core.set_pending_navigation
            />

            {move || if is_mobile.get() {
                view! {
                    <MobileLayout
                        core=core_for_layout.clone()
                        active_view=active_view
                        set_active_view=set_active_view
                        pinned_views=pinned_views
                        set_pinned_views=set_pinned_views
                        on_home=on_home
                        on_open=on_open
                        on_command=on_command
                    />
                }
                .into_any()
            } else {
                view! {
                    <DesktopLayout
                        core=core_for_layout.clone()
                        layout=desktop_layout
                        active_view=active_view
                        set_active_view=set_active_view
                        pinned_views=pinned_views
                        set_pinned_views=set_pinned_views
                        on_home=on_home
                        on_open=on_open
                        on_command=on_command
                        chat_visible=chat_visible
                    />
                }
                .into_any()
            }}

            {{
                let bottom_bar_core = core.clone();
                move || if !is_mobile.get() {
                    view! { <crate::components::bottom_bar::BottomBar core=bottom_bar_core.clone() /> }
                        .into_any()
                } else {
                    view! {}.into_any()
                }
            }}
            <DisconnectedOverlay status=core.ws.status.into() />
        </div>
    }
}

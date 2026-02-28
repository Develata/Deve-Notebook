// apps/web/src/components/main_layout.rs
//! # Main Layout

use crate::components::activity_bar::SidebarView;
use crate::components::desktop_layout::DesktopLayout;
use crate::components::disconnect_overlay::DisconnectedOverlay;
pub use crate::components::layout_context::{ChatControl, SearchControl};
use crate::components::merge_modal_slot::MergeModalSlot;
use crate::components::mobile_layout::MobileLayout;
use crate::hooks::use_core::use_core;
use crate::hooks::use_ctrl_key::use_ctrl_key;
use crate::hooks::use_layout::use_layout;
use crate::i18n::Locale;
use crate::shortcuts::create_global_shortcut_handler;
use leptos::prelude::*;
use web_sys::UiEvent;

#[component]
pub fn MainLayout() -> impl IntoView {
    let _locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let core = use_core();

    let (
        sidebar_width,
        right_width,
        outer_gutter,
        start_resize_left,
        start_resize_right,
        start_resize_outer_left,
        start_resize_outer_right,
        stop_resize,
        do_resize,
        is_resizing,
    ) = use_layout();
    let desktop_layout = (
        sidebar_width,
        right_width,
        outer_gutter,
        start_resize_left,
        start_resize_right,
        start_resize_outer_left,
        start_resize_outer_right,
        stop_resize,
        do_resize,
        is_resizing,
    );

    use_ctrl_key();

    let (show_search, set_show_search) = signal(false);
    let (search_mode, set_search_mode) = signal(String::new());
    provide_context(SearchControl {
        set_show: set_show_search,
        set_mode: set_search_mode,
    });

    let (show_settings, set_show_settings) = signal(false);
    let (active_view, set_active_view) = signal(SidebarView::Explorer);
    let (pinned_views, set_pinned_views) = signal(SidebarView::all());
    let (chat_visible, set_chat_visible) = signal(true);
    provide_context(ChatControl {
        chat_visible,
        set_chat_visible,
    });

    let handle_keydown = create_global_shortcut_handler(
        show_search.into(),
        set_show_search,
        search_mode.into(),
        set_search_mode,
    );
    window_event_listener(leptos::ev::keydown, handle_keydown.clone());

    let (is_mobile, set_is_mobile) = signal(false);
    let update_is_mobile = move || {
        let width = web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(1024.0);
        set_is_mobile.set(width <= 768.0);
    };
    update_is_mobile();
    window_event_listener(leptos::ev::resize, move |_ev: UiEvent| update_is_mobile());

    let on_settings = Callback::new(move |_| set_show_settings.set(true));
    let on_command = Callback::new(move |_| {
        let is_visible = show_search.get_untracked();
        let mode = search_mode.get_untracked();
        let target_mode = ">".to_string();
        if is_visible && mode == target_mode {
            set_show_search.set(false);
        } else {
            set_search_mode.set(target_mode);
            set_show_search.set(true);
        }
    });
    let on_open = Callback::new(move |_| {
        let is_visible = show_search.get_untracked();
        let mode = search_mode.get_untracked();
        let target_mode = String::new();
        if is_visible && mode == target_mode {
            set_show_search.set(false);
        } else {
            set_search_mode.set(target_mode);
            set_show_search.set(true);
        }
    });
    let set_doc = core.set_current_doc;
    let on_home = Callback::new(move |_| set_doc.set(None));

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

            {move || if !is_mobile.get() {
                view! { <crate::components::bottom_bar::BottomBar status=core.ws.status stats=core.stats /> }
                    .into_any()
            } else {
                view! {}.into_any()
            }}
            <DisconnectedOverlay status=core.ws.status.into() />
        </div>
    }
}

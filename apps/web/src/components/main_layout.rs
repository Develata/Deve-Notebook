// apps/web/src/components/main_layout.rs
//! # Main Layout

use self::main_layout_callbacks::{
    build_home_callback, build_open_callback, toggle_search_callback,
};
use self::main_layout_contexts::{
    provide_chat_control, provide_search_control, use_mobile_breakpoint,
};
use crate::api::ConnectionStatus;
use crate::components::activity_bar::SidebarView;
pub use crate::components::layout_context::{ChatControl, SearchControl};
use crate::components::main_layout_runtime::MainLayoutRuntime;
use crate::hooks::use_core::use_core;
use crate::hooks::use_ctrl_key::use_ctrl_key;
use crate::hooks::use_layout::use_layout;
use crate::i18n::Locale;
use crate::shortcuts::create_global_shortcut_handler;
use leptos::prelude::*;

#[path = "main_layout_callbacks.rs"]
mod main_layout_callbacks;
#[path = "main_layout_contexts.rs"]
mod main_layout_contexts;

#[component]
pub fn MainLayout(on_session_expired: Callback<()>) -> AnyView {
    let _locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let core = use_core();
    let ws_status = core.ws.status;

    Effect::new(move |_| {
        if ws_status.get() == ConnectionStatus::Unauthorized {
            on_session_expired.run(());
        }
    });

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
    provide_search_control(set_show_search, set_search_mode);

    let (show_settings, set_show_settings) = signal(false);
    let (active_view, set_active_view) = signal(SidebarView::Explorer);
    let (pinned_views, set_pinned_views) = signal(SidebarView::all());
    let (chat_visible, set_chat_visible) = signal(true);
    provide_chat_control(chat_visible, set_chat_visible);

    let handle_keydown = create_global_shortcut_handler(
        show_search.into(),
        set_show_search,
        search_mode.into(),
        set_search_mode,
    );
    window_event_listener(leptos::ev::keydown, handle_keydown.clone());

    let is_mobile = use_mobile_breakpoint();

    let on_settings = Callback::new(move |_| set_show_settings.set(true));
    let on_command = toggle_search_callback(
        show_search,
        set_show_search,
        search_mode,
        set_search_mode,
        ">".to_string(),
    );
    let on_open = build_open_callback(show_search, set_show_search, search_mode, set_search_mode);
    let on_home = build_home_callback(
        core.set_current_doc,
        core.set_explicit_home,
        core.current_doc,
        core.pending_local_edits,
        core.set_pending_navigation,
    );

    view! {
        <MainLayoutRuntime
            core=core
            desktop_layout=desktop_layout
            is_mobile=is_mobile
            is_resizing=is_resizing
            do_resize=do_resize
            stop_resize=stop_resize
            show_search=show_search
            set_show_search=set_show_search
            search_mode=search_mode
            show_settings=show_settings
            set_show_settings=set_show_settings
            active_view=active_view
            set_active_view=set_active_view
            pinned_views=pinned_views
            set_pinned_views=set_pinned_views
            chat_visible=chat_visible
            on_home=on_home
            on_open=on_open
            on_command=on_command
            on_settings=on_settings
        />
    }
    .into_any()
}

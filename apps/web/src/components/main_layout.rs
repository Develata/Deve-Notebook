// apps/web/src/components/main_layout.rs
//! plan_ref:
//!   - 08_ui_design_01_web#web-layout-persistence
//!   - 06_repository#repo-scope-runtime
//!
//! # Main Layout

use self::main_layout_callbacks::{
    build_home_callback, build_open_callback, toggle_search_callback,
};
use self::main_layout_contexts::use_mobile_breakpoint;
use self::main_layout_setup::{
    bind_global_shortcuts, init_search_ui_state, init_sidebar_ui_state, watch_session_expired,
};
pub use crate::components::layout_context::{ChatControl, SearchControl};
use crate::components::main_layout_runtime::MainLayoutRuntime;
use crate::hooks::use_core::use_core;
use crate::hooks::use_ctrl_key::use_ctrl_key;
use crate::hooks::use_layout::use_layout;
use crate::i18n::Locale;
use leptos::prelude::*;

#[path = "main_layout_callbacks.rs"]
mod main_layout_callbacks;
#[path = "main_layout_contexts.rs"]
mod main_layout_contexts;
#[path = "main_layout_setup.rs"]
mod main_layout_setup;

#[component]
pub fn MainLayout(on_session_expired: Callback<()>, on_logout: Callback<()>) -> AnyView {
    let _locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let core = use_core();
    watch_session_expired(core.ws.status, on_session_expired);

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

    let search = init_search_ui_state();
    let sidebar = init_sidebar_ui_state();
    bind_global_shortcuts(&search);

    let is_mobile = use_mobile_breakpoint();

    let on_settings = Callback::new(move |_| sidebar.set_show_settings.set(true));
    let on_command = toggle_search_callback(
        search.show_search,
        search.set_show_search,
        search.search_mode,
        search.set_search_mode,
        ">".to_string(),
    );
    let on_open = build_open_callback(
        search.show_search,
        search.set_show_search,
        search.search_mode,
        search.set_search_mode,
    );
    let on_home = build_home_callback(
        core.set_current_doc,
        core.set_explicit_home,
        core.current_doc,
        core.current_repo_id,
        core.current_scope_nonce,
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
            show_search=search.show_search
            set_show_search=search.set_show_search
            search_mode=search.search_mode
            show_settings=sidebar.show_settings
            set_show_settings=sidebar.set_show_settings
            active_view=sidebar.active_view
            set_active_view=sidebar.set_active_view
            pinned_views=sidebar.pinned_views
            set_pinned_views=sidebar.set_pinned_views
            chat_visible=sidebar.chat_visible
            on_home=on_home
            on_open=on_open
            on_command=on_command
            on_settings=on_settings
            on_logout=on_logout
        />
    }
    .into_any()
}

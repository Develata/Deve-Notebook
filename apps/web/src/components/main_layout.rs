// apps/web/src/components/main_layout.rs
//! # Main Layout

use crate::api::ConnectionStatus;
use crate::components::activity_bar::SidebarView;
pub use crate::components::layout_context::{ChatControl, SearchControl};
use crate::components::main_layout_runtime::MainLayoutRuntime;
use crate::hooks::use_core::navigation::{NavigationTarget, guard_navigation};
use crate::hooks::use_core::use_core;
use crate::hooks::use_ctrl_key::use_ctrl_key;
use crate::hooks::use_layout::use_layout;
use crate::i18n::Locale;
use crate::shortcuts::create_global_shortcut_handler;
use leptos::prelude::*;
use web_sys::UiEvent;

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
    let set_explicit_home = core.set_explicit_home;
    let current_doc = core.current_doc;
    let pending_local_edits = core.pending_local_edits;
    let set_pending_navigation = core.set_pending_navigation;
    let on_home = Callback::new(move |_| {
        let action = Callback::new(move |_: ()| {
            set_explicit_home.set(true);
            set_doc.set(None);
        });
        let _ = guard_navigation(
            current_doc.get_untracked(),
            &pending_local_edits.get_untracked(),
            set_pending_navigation,
            NavigationTarget::Home,
            action,
        );
    });

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

//! plan_ref:
//!   - 09_auth#unauthorized-disconnected-ui
//!

use super::contexts::{provide_chat_control, provide_search_control};
use crate::api::ConnectionStatus;
use crate::components::activity_bar::SidebarView;
use crate::shortcuts::create_global_shortcut_handler;
use leptos::prelude::*;

pub struct SearchUiState {
    pub show_search: ReadSignal<bool>,
    pub set_show_search: WriteSignal<bool>,
    pub search_mode: ReadSignal<String>,
    pub set_search_mode: WriteSignal<String>,
}

pub struct SidebarUiState {
    pub show_settings: ReadSignal<bool>,
    pub set_show_settings: WriteSignal<bool>,
    pub active_view: ReadSignal<SidebarView>,
    pub set_active_view: WriteSignal<SidebarView>,
    pub pinned_views: ReadSignal<Vec<SidebarView>>,
    pub set_pinned_views: WriteSignal<Vec<SidebarView>>,
    pub chat_visible: ReadSignal<bool>,
}

pub fn watch_session_expired(
    ws_status: ReadSignal<ConnectionStatus>,
    on_session_expired: Callback<()>,
) {
    Effect::new(move |_| {
        if ws_status.get() == ConnectionStatus::Unauthorized {
            on_session_expired.run(());
        }
    });
}

pub fn init_search_ui_state() -> SearchUiState {
    let (show_search, set_show_search) = signal(false);
    let (search_mode, set_search_mode) = signal(String::new());
    provide_search_control(set_show_search, set_search_mode);

    SearchUiState {
        show_search,
        set_show_search,
        search_mode,
        set_search_mode,
    }
}

pub fn init_sidebar_ui_state() -> SidebarUiState {
    let (show_settings, set_show_settings) = signal(false);
    let (active_view, set_active_view) = signal(SidebarView::Explorer);
    let (pinned_views, set_pinned_views) = signal(SidebarView::all());
    let (chat_visible, set_chat_visible) = signal(true);
    provide_chat_control(chat_visible, set_chat_visible);

    SidebarUiState {
        show_settings,
        set_show_settings,
        active_view,
        set_active_view,
        pinned_views,
        set_pinned_views,
        chat_visible,
    }
}

pub fn bind_global_shortcuts(search: &SearchUiState) {
    let handle_keydown = create_global_shortcut_handler(
        search.show_search.into(),
        search.set_show_search,
        search.search_mode.into(),
        search.set_search_mode,
    );
    window_event_listener(leptos::ev::keydown, handle_keydown);
}

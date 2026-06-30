//! plan_ref:
//!   - 08_auth#unauthorized-disconnected-ui
//!

use super::contexts::{
    provide_chat_control, provide_editor_tab_limit_control, provide_outline_control,
    provide_search_control, provide_sidebar_control,
};
use crate::api::ConnectionStatus;
use crate::components::activity_bar::SidebarView;
use crate::components::settings_prefs::{
    persist_ai_chat_visible_preference, persist_max_document_tabs_preference,
    read_ai_chat_visible_preference, read_max_document_tabs_preference,
};
use crate::hooks::use_outline::use_outline;
use crate::i18n::Locale;
use crate::shortcuts::create_global_shortcut_handler;
use crate::storage::prefs::{read_bool_pref, write_bool_pref};
use leptos::prelude::*;

const SIDEBAR_VISIBLE_STORAGE_KEY: &str = "ui_sidebar_visible";

pub struct SearchUiState {
    pub show_search: ReadSignal<bool>,
    pub set_show_search: WriteSignal<bool>,
    pub search_mode: ReadSignal<String>,
    pub set_search_mode: WriteSignal<String>,
}

pub struct OutlineUiState {
    pub set_visible: WriteSignal<bool>,
}

pub struct SidebarUiState {
    pub show_settings: ReadSignal<bool>,
    pub set_show_settings: WriteSignal<bool>,
    pub active_view: ReadSignal<SidebarView>,
    pub set_active_view: WriteSignal<SidebarView>,
    pub pinned_views: ReadSignal<Vec<SidebarView>>,
    pub set_pinned_views: WriteSignal<Vec<SidebarView>>,
    pub chat_visible: ReadSignal<bool>,
    pub visible: ReadSignal<bool>,
    pub set_visible: WriteSignal<bool>,
    pub mobile_visible: ReadSignal<bool>,
    pub set_mobile_visible: WriteSignal<bool>,
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

pub fn init_outline_ui_state() -> OutlineUiState {
    let (visible, set_visible) = use_outline();
    provide_outline_control(visible, set_visible);

    OutlineUiState { set_visible }
}

pub fn init_sidebar_ui_state(is_mobile: ReadSignal<bool>) -> SidebarUiState {
    let (show_settings, set_show_settings) = signal(false);
    let (active_view, set_active_view) = signal(SidebarView::Explorer);
    let (pinned_views, set_pinned_views) = signal(SidebarView::all());
    let (chat_visible, set_chat_visible) = use_chat_visibility();
    let (visible, set_visible) = use_sidebar_visibility();
    let (mobile_visible, set_mobile_visible) = signal(false);
    provide_chat_control(chat_visible, set_chat_visible);
    provide_sidebar_control(is_mobile, set_visible, set_mobile_visible, set_active_view);

    SidebarUiState {
        show_settings,
        set_show_settings,
        active_view,
        set_active_view,
        pinned_views,
        set_pinned_views,
        chat_visible,
        visible,
        set_visible,
        mobile_visible,
        set_mobile_visible,
    }
}

pub fn init_editor_tab_limit_ui_state() -> ReadSignal<usize> {
    let (max_document_tabs, set_max_document_tabs) = use_editor_tab_limit();
    provide_editor_tab_limit_control(max_document_tabs, set_max_document_tabs);

    max_document_tabs
}

pub fn bind_global_shortcuts(
    search: &SearchUiState,
    outline: &OutlineUiState,
    sidebar: &SidebarUiState,
    locale: RwSignal<Locale>,
) {
    let handle_keydown = create_global_shortcut_handler(
        search.show_search.into(),
        search.set_show_search,
        search.search_mode.into(),
        search.set_search_mode,
        locale,
        outline.set_visible,
        sidebar.set_visible,
    );
    window_event_listener(leptos::ev::keydown, handle_keydown);
}

fn use_sidebar_visibility() -> (ReadSignal<bool>, WriteSignal<bool>) {
    let initial = read_bool_pref(SIDEBAR_VISIBLE_STORAGE_KEY).unwrap_or(true);
    let (visible, set_visible) = signal(initial);

    Effect::new(move |_| {
        let _ = write_bool_pref(SIDEBAR_VISIBLE_STORAGE_KEY, visible.get());
    });

    (visible, set_visible)
}

fn use_chat_visibility() -> (ReadSignal<bool>, WriteSignal<bool>) {
    let initial = read_ai_chat_visible_preference();
    let (visible, set_visible) = signal(initial);

    Effect::new(move |_| {
        persist_ai_chat_visible_preference(visible.get());
    });

    (visible, set_visible)
}

fn use_editor_tab_limit() -> (ReadSignal<usize>, WriteSignal<usize>) {
    let initial = read_max_document_tabs_preference();
    let (max_document_tabs, set_max_document_tabs) = signal(initial);

    Effect::new(move |_| {
        persist_max_document_tabs_preference(max_document_tabs.get());
    });

    (max_document_tabs, set_max_document_tabs)
}

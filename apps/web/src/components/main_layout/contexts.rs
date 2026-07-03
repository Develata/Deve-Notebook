//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 16_ai_agent#native-ai-chat-runtime
//!
use super::{ChatControl, EditorTabLimitControl, OutlineControl, SearchControl, SidebarControl};
use crate::components::layout_breakpoint::{current_viewport_width, viewport_width_maps_to_mobile};
use leptos::prelude::*;
use web_sys::UiEvent;

pub fn provide_search_control(
    set_show_search: WriteSignal<bool>,
    set_search_mode: WriteSignal<String>,
) {
    provide_context(SearchControl {
        set_show: set_show_search,
        set_mode: set_search_mode,
    });
}

pub fn provide_chat_control(chat_visible: ReadSignal<bool>, set_chat_visible: WriteSignal<bool>) {
    provide_context(ChatControl {
        chat_visible,
        set_chat_visible,
    });
}

pub fn provide_editor_tab_limit_control(
    max_document_tabs: ReadSignal<usize>,
    set_max_document_tabs: WriteSignal<usize>,
) {
    provide_context(EditorTabLimitControl {
        max_document_tabs,
        set_max_document_tabs,
    });
}

pub fn provide_outline_control(visible: ReadSignal<bool>, set_visible: WriteSignal<bool>) {
    provide_context(OutlineControl {
        visible,
        set_visible,
    });
}

pub fn provide_sidebar_control(
    is_mobile: ReadSignal<bool>,
    set_visible: WriteSignal<bool>,
    set_mobile_visible: WriteSignal<bool>,
    set_active_view: WriteSignal<crate::components::activity_bar::SidebarView>,
) {
    provide_context(SidebarControl {
        is_mobile,
        set_visible,
        set_mobile_visible,
        set_active_view,
    });
}

pub fn use_mobile_breakpoint() -> ReadSignal<bool> {
    let (is_mobile, set_is_mobile) = signal(false);
    let update_is_mobile = move || {
        let width = current_viewport_width().unwrap_or(1024.0);
        set_is_mobile.set(viewport_width_maps_to_mobile(width));
    };
    update_is_mobile();
    window_event_listener(leptos::ev::resize, move |_ev: UiEvent| update_is_mobile());
    is_mobile
}

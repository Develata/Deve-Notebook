//! plan_ref:
//!   - 08_ui_design_01_web#web-layout-persistence
//!   - 10_ai_agent#native-ai-chat-runtime
//!
use super::{ChatControl, SearchControl};
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

pub fn use_mobile_breakpoint() -> ReadSignal<bool> {
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
    is_mobile
}

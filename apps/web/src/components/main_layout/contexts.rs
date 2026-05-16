//! plan_ref:
//!   - 08_ui_design_01_web#web-layout-persistence
//!   - 08_ui_design_03_mobile#mobile-responsive-layout
//!   - 10_ai_agent#native-ai-chat-runtime
//!
use super::{ChatControl, OutlineControl, SearchControl, SidebarControl};
use leptos::prelude::*;
use web_sys::UiEvent;

pub(crate) const MOBILE_BREAKPOINT_WIDTH: f64 = 768.0;

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

pub fn provide_outline_control(visible: ReadSignal<bool>, set_visible: WriteSignal<bool>) {
    provide_context(OutlineControl {
        visible,
        set_visible,
    });
}

pub fn provide_sidebar_control(set_visible: WriteSignal<bool>) {
    provide_context(SidebarControl { set_visible });
}

pub(crate) fn viewport_width_maps_to_mobile(width: f64) -> bool {
    width <= MOBILE_BREAKPOINT_WIDTH
}

pub fn use_mobile_breakpoint() -> ReadSignal<bool> {
    let (is_mobile, set_is_mobile) = signal(false);
    let update_is_mobile = move || {
        let width = web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(1024.0);
        set_is_mobile.set(viewport_width_maps_to_mobile(width));
    };
    update_is_mobile();
    window_event_listener(leptos::ev::resize, move |_ev: UiEvent| update_is_mobile());
    is_mobile
}

#[cfg(test)]
mod tests {
    use super::{MOBILE_BREAKPOINT_WIDTH, viewport_width_maps_to_mobile};

    #[test]
    fn mobile_viewport_mapping_uses_inclusive_768px_boundary() {
        assert!(viewport_width_maps_to_mobile(375.0));
        assert!(viewport_width_maps_to_mobile(MOBILE_BREAKPOINT_WIDTH));
        assert!(!viewport_width_maps_to_mobile(
            MOBILE_BREAKPOINT_WIDTH + 0.1
        ));
        assert!(!viewport_width_maps_to_mobile(1024.0));
    }
}

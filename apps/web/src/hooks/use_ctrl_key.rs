// apps/web/src/hooks/use_ctrl_key.rs
//! plan_ref:
//!   - 15_settings#keyboard-shortcuts
//!
//! Global Ctrl/Meta key state management for link navigation.
//!
//! This hook monitors keydown/keyup events and toggles `is-ctrl-pressed`
//! class on the document body, enabling CSS-driven link activation.

use leptos::prelude::*;

/// Sets up global Ctrl/Meta key listeners.
///
/// # Behavior
/// - Adds `is-ctrl-pressed` class to `<body>` when Ctrl or Meta is pressed.
/// - Removes the class when the key is released or window loses focus.
///
/// # Implementation Notes
/// - Uses CSS class toggle for zero-copy, high-performance visual feedback.
/// - Handles edge cases: blur event clears state to prevent stuck modifier.
pub fn use_ctrl_key() {
    // Keydown: Add class when Ctrl/Meta pressed
    window_event_listener(leptos::ev::keydown, move |ev: web_sys::KeyboardEvent| {
        if should_mark_ctrl_pressed(ev.ctrl_key(), ev.meta_key())
            && let Some(body) = body()
        {
            let _ = body.class_list().add_1("is-ctrl-pressed");
        }
    });

    // Keyup: Remove class when Ctrl/Meta released
    window_event_listener(leptos::ev::keyup, move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        if should_clear_ctrl_pressed_on_keyup(&key, ev.ctrl_key(), ev.meta_key())
            && let Some(body) = body()
        {
            let _ = body.class_list().remove_1("is-ctrl-pressed");
        }
    });

    // Blur: Clear state when window loses focus (edge case protection)
    window_event_listener(leptos::ev::blur, move |_| {
        if let Some(body) = body() {
            let _ = body.class_list().remove_1("is-ctrl-pressed");
        }
    });
}

fn should_mark_ctrl_pressed(ctrl: bool, meta: bool) -> bool {
    ctrl || meta
}

fn should_clear_ctrl_pressed_on_keyup(key: &str, ctrl: bool, meta: bool) -> bool {
    (key == "Control" || key == "Meta") && !should_mark_ctrl_pressed(ctrl, meta)
}

fn body() -> Option<web_sys::HtmlElement> {
    document()?.body()
}

fn document() -> Option<web_sys::Document> {
    browser_window()?.document()
}

#[cfg(target_arch = "wasm32")]
fn browser_window() -> Option<web_sys::Window> {
    web_sys::window()
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_window() -> Option<web_sys::Window> {
    None
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{body, document, should_clear_ctrl_pressed_on_keyup, should_mark_ctrl_pressed};

    #[test]
    fn ctrl_key_dom_helpers_fail_soft_without_browser_window() {
        assert!(document().is_none());
        assert!(body().is_none());
    }

    #[test]
    fn ctrl_key_marker_tracks_either_modifier() {
        assert!(!should_mark_ctrl_pressed(false, false));
        assert!(should_mark_ctrl_pressed(true, false));
        assert!(should_mark_ctrl_pressed(false, true));
    }

    #[test]
    fn ctrl_key_keyup_keeps_marker_while_other_modifier_is_still_pressed() {
        assert!(should_clear_ctrl_pressed_on_keyup("Control", false, false));
        assert!(should_clear_ctrl_pressed_on_keyup("Meta", false, false));
        assert!(!should_clear_ctrl_pressed_on_keyup("Control", false, true));
        assert!(!should_clear_ctrl_pressed_on_keyup("Meta", true, false));
        assert!(!should_clear_ctrl_pressed_on_keyup("Shift", false, false));
    }
}

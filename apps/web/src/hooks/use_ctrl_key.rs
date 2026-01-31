// apps/web/src/hooks/use_ctrl_key.rs
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
    window_event_listener(leptos::ev::keydown, move |ev| {
        let ev: web_sys::KeyboardEvent = ev.into();
        if ev.ctrl_key() || ev.meta_key() {
            if let Some(body) = document().body() {
                let _ = body.class_list().add_1("is-ctrl-pressed");
            }
        }
    });

    // Keyup: Remove class when Ctrl/Meta released
    window_event_listener(leptos::ev::keyup, move |ev| {
        let ev: web_sys::KeyboardEvent = ev.into();
        let key = ev.key();
        if key == "Control" || key == "Meta" {
            if let Some(body) = document().body() {
                let _ = body.class_list().remove_1("is-ctrl-pressed");
            }
        }
    });

    // Blur: Clear state when window loses focus (edge case protection)
    window_event_listener(leptos::ev::blur, move |_| {
        if let Some(body) = document().body() {
            let _ = body.class_list().remove_1("is-ctrl-pressed");
        }
    });
}

/// Helper: Get document from window.
fn document() -> web_sys::Document {
    web_sys::window()
        .expect("window")
        .document()
        .expect("document")
}

// apps/web/src/hooks/use_outline.rs
//! Persistent Outline visibility state management.
//!
//! This hook manages the Outline panel visibility with LocalStorage persistence.
//! The state survives across document switches and page reloads.

use leptos::prelude::*;

const STORAGE_KEY: &str = "ui_outline_visible";

/// Returns (is_visible, set_visible) for Outline panel.
///
/// # Persistence
/// - Reads initial state from `localStorage` on mount.
/// - Writes state changes to `localStorage` automatically.
///
/// # Default
/// - If no stored value exists, defaults to `true` (visible).
pub fn use_outline() -> (ReadSignal<bool>, WriteSignal<bool>) {
    // 1. Read initial state from LocalStorage
    let initial = read_from_storage().unwrap_or(true);

    let (visible, set_visible) = signal(initial);

    // 2. Persist changes to LocalStorage
    Effect::new(move |_| {
        let val = visible.get();
        write_to_storage(val);
    });

    (visible, set_visible)
}

/// Reads boolean value from LocalStorage.
fn read_from_storage() -> Option<bool> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let val = storage.get_item(STORAGE_KEY).ok()??;
    Some(val == "true")
}

/// Writes boolean value to LocalStorage.
fn write_to_storage(val: bool) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(STORAGE_KEY, if val { "true" } else { "false" });
        }
    }
}

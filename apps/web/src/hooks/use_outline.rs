// apps/web/src/hooks/use_outline.rs
//! plan_ref:
//!   - 11_ui_design_01_web#web-layout-persistence
//!
//! Persistent Outline visibility state management.
//!
//! This hook manages the Outline panel visibility with LocalStorage persistence.
//! The state survives across document switches and page reloads.

use crate::storage::prefs::{read_bool_pref, write_bool_pref};
use leptos::prelude::*;

const STORAGE_KEY: &str = "ui_outline_visible";

/// Returns (is_visible, set_visible) for Outline panel.
///
/// # Persistence
/// - Reads initial state from the UI prefs fallback layer on mount.
/// - Writes state changes to the UI prefs fallback layer automatically.
///
/// # Default
/// - If no stored value exists, defaults to `true` (visible).
pub fn use_outline() -> (ReadSignal<bool>, WriteSignal<bool>) {
    // 1. Read initial state from the UI prefs fallback layer.
    let initial = read_from_storage().unwrap_or(true);

    let (visible, set_visible) = signal(initial);

    // 2. Persist changes through the UI prefs fallback layer.
    Effect::new(move |_| {
        let val = visible.get();
        write_to_storage(val);
    });

    (visible, set_visible)
}

/// Reads boolean value from UI prefs.
fn read_from_storage() -> Option<bool> {
    read_bool_pref(STORAGE_KEY)
}

/// Writes boolean value to UI prefs.
fn write_to_storage(val: bool) {
    let _ = write_bool_pref(STORAGE_KEY, val);
}

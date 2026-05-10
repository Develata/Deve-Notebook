//! plan_ref:
//!   - 12_commands#command-palette-shortcuts
//!   - 13_settings#keyboard-shortcuts

use leptos::prelude::*;
use web_sys::KeyboardEvent;

pub(super) fn handle_search_box_shortcut(
    ev: &KeyboardEvent,
    set_show: WriteSignal<bool>,
    query: Signal<String>,
    set_query: WriteSignal<String>,
    set_selected_index: WriteSignal<usize>,
    input_ref: NodeRef<leptos::html::Input>,
) {
    let key = ev.key();
    let is_ctrl = ev.ctrl_key() || ev.meta_key();
    let shift = ev.shift_key();
    let key_lower = key.to_lowercase();

    if key == "Escape" {
        ev.prevent_default();
        ev.stop_propagation();
        set_show.set(false);
        return;
    }

    if is_ctrl && key_lower == "p" {
        ev.prevent_default();
        ev.stop_propagation();

        if shift {
            if query.get_untracked().starts_with('>') {
                set_show.set(false);
            } else {
                set_query.set(">".to_string());
                set_selected_index.set(0);
                focus_input(input_ref);
            }
        } else {
            let q = query.get_untracked();
            let is_file = !q.starts_with('>') && !q.starts_with('@');
            if is_file {
                set_show.set(false);
            } else {
                set_query.set(String::new());
                set_selected_index.set(0);
                focus_input(input_ref);
            }
        }
    }
}

fn focus_input(input_ref: NodeRef<leptos::html::Input>) {
    if let Some(el) = input_ref.get_untracked() {
        let _ = el.focus();
    }
}

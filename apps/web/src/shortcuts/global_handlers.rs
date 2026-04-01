use leptos::prelude::*;
use web_sys::KeyboardEvent;

pub(super) fn handle_global_shortcut(
    ev: &KeyboardEvent,
    show_search: Signal<bool>,
    set_show_search: WriteSignal<bool>,
    search_mode: Signal<String>,
    set_search_mode: WriteSignal<String>,
) {
    let is_ctrl = ev.meta_key() || ev.ctrl_key();
    let shift = ev.shift_key();
    let key = ev.key().to_lowercase();

    if is_ctrl && shift && key == "p" {
        ev.prevent_default();
        ev.stop_propagation();

        if show_search.get() && search_mode.get() == ">" {
            set_show_search.set(false);
        } else {
            set_search_mode.set(">".to_string());
            set_show_search.set(true);
        }
        return;
    }

    if is_ctrl && !shift && key == "p" {
        ev.prevent_default();
        ev.stop_propagation();

        if show_search.get() && search_mode.get().is_empty() {
            set_show_search.set(false);
        } else {
            set_search_mode.set(String::new());
            set_show_search.set(true);
        }
    }
}

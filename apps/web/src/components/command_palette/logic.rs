//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 11_ui_design_01_web#web-layout-persistence
//!
use super::registry::{create_static_commands, filter_commands};
use super::types::Command;
use crate::components::focus_scope;
use crate::i18n::Locale;
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::KeyboardEvent;

pub(super) fn attach_reset_effect(
    show: Signal<bool>,
    set_query: WriteSignal<String>,
    set_selected_index: WriteSignal<usize>,
) {
    Effect::new(move |_| {
        if show.get() {
            set_query.set(String::new());
            set_selected_index.set(0);
        }
    });
}

pub(super) fn attach_focus_restore_effect(
    show: Signal<bool>,
    input_ref: NodeRef<leptos::html::Input>,
) {
    let last_show = StoredValue::new_local(show.get_untracked());
    let previous_focus = StoredValue::new_local(None::<web_sys::Element>);

    Effect::new(move |_| {
        let open = show.get();
        let was_open = last_show.get_value();
        last_show.set_value(open);

        if open && !was_open {
            previous_focus.set_value(focus_scope::active_element());
            focus_scope::focus_input_next_frame(input_ref);
        } else if !open && was_open {
            let previous = previous_focus.get_value();
            previous_focus.set_value(None);
            focus_scope::restore_focus_next_frame(previous);
        }
    });
}

pub(super) fn create_filtered_commands_memo(
    query: Signal<String>,
    locale: RwSignal<Locale>,
    on_settings: Callback<()>,
    on_open: Callback<()>,
    set_show: WriteSignal<bool>,
) -> Memo<Vec<Command>> {
    Memo::new(move |_| {
        let q = query.get();
        let current_locale = locale.get();
        let static_cmds =
            create_static_commands(current_locale, on_settings, on_open, set_show, locale);
        filter_commands(&q, static_cmds, 50)
    })
}

pub(super) fn make_active_index(
    selected_index: Signal<usize>,
    filtered_commands: Memo<Vec<Command>>,
) -> impl Fn() -> usize + Copy + Send + Sync + 'static {
    move || {
        let count = filtered_commands.get().len();
        if count == 0 {
            return 0;
        }
        let current = selected_index.get();
        if current >= count { 0 } else { current }
    }
}

pub(super) fn build_keydown_handler(
    filtered_commands: Memo<Vec<Command>>,
    selected_index: Signal<usize>,
    set_selected_index: WriteSignal<usize>,
    set_show: WriteSignal<bool>,
    active_index: Arc<dyn Fn() -> usize + Send + Sync>,
) -> impl Fn(KeyboardEvent) + Send + Sync + 'static {
    move |ev: KeyboardEvent| {
        let key = ev.key();
        if (ev.ctrl_key() || ev.meta_key()) && key == "k" {
            ev.prevent_default();
            ev.stop_propagation();
            set_show.set(false);
            return;
        }

        let count = filtered_commands.get().len();
        if count == 0 {
            return;
        }

        match key.as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                set_selected_index.update(|i| *i = (*i + 1) % count);
            }
            "ArrowUp" => {
                ev.prevent_default();
                set_selected_index.update(|i| *i = (*i + count - 1) % count);
            }
            "Enter" => {
                ev.prevent_default();
                let idx = active_index();
                if let Some(cmd) = filtered_commands.get().get(idx) {
                    cmd.action.run(());
                }
            }
            "Escape" => {
                ev.prevent_default();
                set_show.set(false);
            }
            _ => {
                let _ = selected_index;
            }
        }
    }
}

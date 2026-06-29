//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use super::registry::{create_static_commands, filter_commands};
use super::types::Command;
use crate::components::focus_scope;
use crate::components::main_layout::SidebarControl;
use crate::hooks::use_core::{SourceControlContext, source_control_notice::SourceControlNotice};
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
    set_source_control_notice: Option<WriteSignal<Option<SourceControlNotice>>>,
) -> Memo<Vec<Command>> {
    let set_source_control_notice = set_source_control_notice.or_else(|| {
        use_context::<SourceControlContext>().map(|source_control| source_control.set_notice)
    });
    let sidebar_control = use_context::<SidebarControl>();
    Memo::new(move |_| {
        let q = query.get();
        let current_locale = locale.get();
        let static_cmds = create_static_commands(
            current_locale,
            on_settings,
            on_open,
            set_show,
            locale,
            set_source_control_notice,
            sidebar_control,
        );
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

#[cfg(test)]
mod tests {
    use super::create_filtered_commands_memo;
    use crate::i18n::Locale;
    use leptos::prelude::{Callback, GetUntracked, RwSignal, Set, signal};
    use leptos::reactive::owner::Owner;

    #[test]
    fn filtered_command_actions_survive_memo_recompute() {
        let owner = Owner::new();

        owner.with(|| {
            let (query, set_query) = signal(String::new());
            let (show_palette, set_show_palette) = signal(true);
            let (settings_opened, set_settings_opened) = signal(false);
            let locale = RwSignal::new(Locale::En);
            let commands = create_filtered_commands_memo(
                query.into(),
                locale,
                Callback::new(move |_| set_settings_opened.set(true)),
                Callback::new(|_| {}),
                set_show_palette,
                None,
            );
            let settings = commands
                .get_untracked()
                .into_iter()
                .find(|command| command.id == "settings")
                .expect("settings command");

            set_query.set("lang".to_string());
            let _ = commands.get_untracked();
            settings.action.run(());

            assert!(settings_opened.get_untracked());
            assert!(!show_palette.get_untracked());
        });
    }
}

//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 15_settings#keyboard-shortcuts

use leptos::prelude::*;
use web_sys::KeyboardEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchBoxShortcutAction {
    Close,
    CommandMode,
    FileMode,
    BranchMode,
}

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
    let action = plan_search_box_shortcut(is_ctrl, ev.shift_key(), &key, &query.get_untracked());

    match action {
        Some(SearchBoxShortcutAction::Close) => {
            ev.prevent_default();
            ev.stop_propagation();
            set_show.set(false);
        }
        Some(SearchBoxShortcutAction::CommandMode) => {
            ev.prevent_default();
            ev.stop_propagation();
            set_query.set(">".to_string());
            set_selected_index.set(0);
            focus_input(input_ref);
        }
        Some(SearchBoxShortcutAction::FileMode) => {
            ev.prevent_default();
            ev.stop_propagation();
            set_query.set(String::new());
            set_selected_index.set(0);
            focus_input(input_ref);
        }
        Some(SearchBoxShortcutAction::BranchMode) => {
            ev.prevent_default();
            ev.stop_propagation();
            set_query.set("@".to_string());
            set_selected_index.set(0);
            focus_input(input_ref);
        }
        None => {}
    }
}

fn plan_search_box_shortcut(
    is_ctrl: bool,
    shift: bool,
    key: &str,
    query: &str,
) -> Option<SearchBoxShortcutAction> {
    let key_lower = key.to_lowercase();

    if key == "Escape" {
        return Some(SearchBoxShortcutAction::Close);
    }

    if is_ctrl && shift && key_lower == "k" {
        if query.starts_with('@') {
            Some(SearchBoxShortcutAction::Close)
        } else {
            Some(SearchBoxShortcutAction::BranchMode)
        }
    } else if is_ctrl && key_lower == "p" {
        if shift {
            if query.starts_with('>') {
                Some(SearchBoxShortcutAction::Close)
            } else {
                Some(SearchBoxShortcutAction::CommandMode)
            }
        } else {
            let is_file = !query.starts_with('>') && !query.starts_with('@');
            if is_file {
                Some(SearchBoxShortcutAction::Close)
            } else {
                Some(SearchBoxShortcutAction::FileMode)
            }
        }
    } else {
        None
    }
}

fn focus_input(input_ref: NodeRef<leptos::html::Input>) {
    if let Some(el) = input_ref.get_untracked() {
        let _ = el.focus();
    }
}

#[cfg(test)]
mod tests {
    use super::{SearchBoxShortcutAction, plan_search_box_shortcut};

    #[test]
    fn search_box_shortcut_routes_branch_switcher_while_input_has_focus() {
        assert_eq!(
            plan_search_box_shortcut(true, true, "K", ">settings"),
            Some(SearchBoxShortcutAction::BranchMode)
        );
    }

    #[test]
    fn search_box_shortcut_keeps_existing_command_and_file_toggles() {
        assert_eq!(
            plan_search_box_shortcut(true, true, "P", "file.md"),
            Some(SearchBoxShortcutAction::CommandMode)
        );
        assert_eq!(
            plan_search_box_shortcut(true, true, "P", ">settings"),
            Some(SearchBoxShortcutAction::Close)
        );
        assert_eq!(
            plan_search_box_shortcut(true, false, "p", "@"),
            Some(SearchBoxShortcutAction::FileMode)
        );
        assert_eq!(
            plan_search_box_shortcut(true, true, "k", "@peer"),
            Some(SearchBoxShortcutAction::Close)
        );
    }
}

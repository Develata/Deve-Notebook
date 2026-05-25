//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 15_settings#keyboard-shortcuts

use crate::i18n::{Locale, persist_locale_preference};
use leptos::prelude::*;
use web_sys::KeyboardEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GlobalShortcutAction {
    CommandPalette,
    BranchSwitcher,
    GoToFile,
    ToggleLanguage,
    ToggleOutline,
    ToggleSidebar,
}

pub(super) fn plan_global_shortcut(
    is_ctrl: bool,
    shift: bool,
    alt: bool,
    key: &str,
) -> Option<GlobalShortcutAction> {
    if !is_ctrl || alt {
        return None;
    }

    match (shift, key) {
        (true, "p") => Some(GlobalShortcutAction::CommandPalette),
        (true, "k") => Some(GlobalShortcutAction::BranchSwitcher),
        (false, "p") => Some(GlobalShortcutAction::GoToFile),
        (false, "l") => Some(GlobalShortcutAction::ToggleLanguage),
        (true, "o") => Some(GlobalShortcutAction::ToggleOutline),
        (false, "b") => Some(GlobalShortcutAction::ToggleSidebar),
        _ => None,
    }
}

pub(super) fn handle_global_shortcut(ev: &KeyboardEvent, signals: GlobalShortcutSignals) {
    let is_ctrl = ev.meta_key() || ev.ctrl_key();
    let shift = ev.shift_key();
    let alt = ev.alt_key();
    let key = ev.key().to_lowercase();
    let Some(action) = plan_global_shortcut(is_ctrl, shift, alt, &key) else {
        return;
    };

    ev.prevent_default();
    ev.stop_propagation();

    apply_global_shortcut_action(action, signals);
}

#[derive(Clone, Copy)]
pub(super) struct GlobalShortcutSignals {
    pub show_search: Signal<bool>,
    pub set_show_search: WriteSignal<bool>,
    pub search_mode: Signal<String>,
    pub set_search_mode: WriteSignal<String>,
    pub locale: RwSignal<Locale>,
    pub set_outline_visible: WriteSignal<bool>,
    pub set_sidebar_visible: WriteSignal<bool>,
}

pub(super) fn apply_global_shortcut_action(
    action: GlobalShortcutAction,
    signals: GlobalShortcutSignals,
) {
    match action {
        GlobalShortcutAction::CommandPalette => {
            toggle_search_mode(signals, ">");
        }
        GlobalShortcutAction::BranchSwitcher => {
            toggle_search_mode(signals, "@");
        }
        GlobalShortcutAction::GoToFile => {
            toggle_search_mode(signals, "");
        }
        GlobalShortcutAction::ToggleLanguage => {
            signals.locale.update(|locale| {
                *locale = locale.toggle();
                persist_locale_preference(*locale);
            });
        }
        GlobalShortcutAction::ToggleOutline => {
            signals
                .set_outline_visible
                .update(|visible| *visible = !*visible);
        }
        GlobalShortcutAction::ToggleSidebar => {
            signals
                .set_sidebar_visible
                .update(|visible| *visible = !*visible);
        }
    }
}

fn toggle_search_mode(signals: GlobalShortcutSignals, mode: &'static str) {
    if signals.show_search.get_untracked() && signals.search_mode.get_untracked() == mode {
        signals.set_show_search.set(false);
    } else {
        signals.set_search_mode.set(mode.to_string());
        signals.set_show_search.set(true);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GlobalShortcutAction, GlobalShortcutSignals, apply_global_shortcut_action,
        plan_global_shortcut,
    };
    use crate::i18n::Locale;
    use leptos::prelude::*;

    #[test]
    fn global_shortcut_plan_covers_settings_shortcuts() {
        assert_eq!(
            plan_global_shortcut(true, false, false, "l"),
            Some(GlobalShortcutAction::ToggleLanguage)
        );
        assert_eq!(
            plan_global_shortcut(true, true, false, "o"),
            Some(GlobalShortcutAction::ToggleOutline)
        );
        assert_eq!(
            plan_global_shortcut(true, false, false, "b"),
            Some(GlobalShortcutAction::ToggleSidebar)
        );
        assert_eq!(
            plan_global_shortcut(true, false, true, "b"),
            None,
            "Alt-modified shortcuts must not collide with browser or OS bindings"
        );
    }

    #[test]
    fn global_shortcut_actions_toggle_layout_and_language_state() {
        let owner = leptos::reactive::owner::Owner::new();

        owner.with(|| {
            let (show_search, set_show_search) = signal(false);
            let (search_mode, set_search_mode) = signal(String::new());
            let locale = RwSignal::new(Locale::En);
            let (outline_visible, set_outline_visible) = signal(true);
            let (sidebar_visible, set_sidebar_visible) = signal(true);
            let signals = GlobalShortcutSignals {
                show_search: show_search.into(),
                set_show_search,
                search_mode: search_mode.into(),
                set_search_mode,
                locale,
                set_outline_visible,
                set_sidebar_visible,
            };

            apply_global_shortcut_action(GlobalShortcutAction::ToggleLanguage, signals);
            apply_global_shortcut_action(GlobalShortcutAction::ToggleOutline, signals);
            apply_global_shortcut_action(GlobalShortcutAction::ToggleSidebar, signals);

            assert_eq!(locale.get_untracked(), Locale::Zh);
            assert!(!outline_visible.get_untracked());
            assert!(!sidebar_visible.get_untracked());
        });
    }

    #[test]
    fn global_shortcut_actions_preserve_existing_search_modes() {
        let owner = leptos::reactive::owner::Owner::new();

        owner.with(|| {
            let (show_search, set_show_search) = signal(false);
            let (search_mode, set_search_mode) = signal(String::new());
            let locale = RwSignal::new(Locale::En);
            let (_, set_outline_visible) = signal(true);
            let (_, set_sidebar_visible) = signal(true);
            let signals = GlobalShortcutSignals {
                show_search: show_search.into(),
                set_show_search,
                search_mode: search_mode.into(),
                set_search_mode,
                locale,
                set_outline_visible,
                set_sidebar_visible,
            };

            apply_global_shortcut_action(GlobalShortcutAction::CommandPalette, signals);
            assert!(show_search.get_untracked());
            assert_eq!(search_mode.get_untracked(), ">");

            apply_global_shortcut_action(GlobalShortcutAction::CommandPalette, signals);
            assert!(!show_search.get_untracked());

            apply_global_shortcut_action(GlobalShortcutAction::BranchSwitcher, signals);
            assert!(show_search.get_untracked());
            assert_eq!(search_mode.get_untracked(), "@");

            apply_global_shortcut_action(GlobalShortcutAction::GoToFile, signals);
            assert!(show_search.get_untracked());
            assert_eq!(search_mode.get_untracked(), "");
        });
    }
}

//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!
use crate::components::command_palette::types::Command;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

fn disabled_command(id: &'static str, title: &'static str, reason: &'static str) -> Command {
    Command::unavailable(id, title, reason, Callback::new(|_| {}))
}

pub(super) fn source_control_reserved_commands(locale: Locale) -> Vec<Command> {
    let reason = (t::command_palette::source_control_panel_reason)(locale);
    vec![
        disabled_command(
            "source_control_sync",
            (t::command_palette::source_control_sync)(locale),
            reason,
        ),
        disabled_command(
            "source_control_commit",
            (t::command_palette::source_control_commit)(locale),
            reason,
        ),
        disabled_command(
            "source_control_push",
            (t::command_palette::source_control_push)(locale),
            reason,
        ),
    ]
}

pub(super) fn ai_reserved_commands(locale: Locale) -> Vec<Command> {
    vec![
        disabled_command(
            "ai_retry_last_request",
            (t::command_palette::ai_retry_last_request)(locale),
            (t::command_palette::ai_retry_panel_reason)(locale),
        ),
        disabled_command(
            "ai_switch_backend",
            (t::command_palette::ai_switch_backend)(locale),
            (t::command_palette::ai_backend_settings_reason)(locale),
        ),
        disabled_command(
            "ai_switch_plan",
            (t::command_palette::ai_switch_plan)(locale),
            (t::command_palette::ai_slash_mode_reason)(locale),
        ),
        disabled_command(
            "ai_switch_build",
            (t::command_palette::ai_switch_build)(locale),
            (t::command_palette::ai_slash_mode_reason)(locale),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{ai_reserved_commands, source_control_reserved_commands};
    use crate::i18n::Locale;

    #[test]
    fn reserved_commands_are_unavailable_entries() {
        let commands = [
            source_control_reserved_commands(Locale::En),
            ai_reserved_commands(Locale::En),
        ]
        .concat();

        assert!(
            commands
                .iter()
                .all(|command| command.availability.is_unavailable())
        );
        assert!(
            commands
                .iter()
                .any(|command| command.id == "source_control_commit")
        );
        assert!(
            commands
                .iter()
                .any(|command| command.id == "ai_switch_build")
        );
    }
}

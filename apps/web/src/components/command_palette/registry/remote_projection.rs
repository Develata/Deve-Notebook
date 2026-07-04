//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 05_diff_logic#remote-projection-transport
//!
//! Web command entries for remote projection transport intents.

use crate::components::command_palette::types::Command;
use crate::i18n::{Locale, t};

pub(super) fn remote_projection_commands(locale: Locale) -> Vec<Command> {
    let reason = (t::command_palette::remote_projection_cli_only_reason)(locale);
    let group = (t::command_palette::group_remote_projection)(locale);
    vec![
        remote_projection_command(
            "webdav:push",
            (t::command_palette::webdav_push)(locale),
            reason,
            group,
        ),
        remote_projection_command(
            "webdav:pull",
            (t::command_palette::webdav_pull)(locale),
            reason,
            group,
        ),
        remote_projection_command(
            "s3:push",
            (t::command_palette::s3_push)(locale),
            reason,
            group,
        ),
        remote_projection_command(
            "s3:pull",
            (t::command_palette::s3_pull)(locale),
            reason,
            group,
        ),
    ]
}

fn remote_projection_command(
    id: &'static str,
    title: &'static str,
    reason: &'static str,
    group: &'static str,
) -> Command {
    Command::unavailable(id, title, reason, || {})
        .with_group(group)
        .with_enabled_when(reason)
}

#[cfg(test)]
mod tests {
    use super::remote_projection_commands;
    use crate::i18n::Locale;

    #[test]
    fn remote_projection_commands_are_cli_backend_only_entries() {
        let commands = remote_projection_commands(Locale::En);
        let ids = commands
            .iter()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, ["webdav:push", "webdav:pull", "s3:push", "s3:pull"]);
        assert!(
            commands
                .iter()
                .all(|command| command.availability.is_unavailable())
        );
        assert!(commands[1].title.contains("webdav:pull"));
        assert!(commands[1].detail_text().contains("External Changes"));
    }
}

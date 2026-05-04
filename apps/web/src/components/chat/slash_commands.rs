//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChatSessionMode {
    Plan,
    Build,
}

impl ChatSessionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ChatSessionMode::Plan => "plan",
            ChatSessionMode::Build => "build",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlashCommand {
    Plan,
    Build,
    Agents,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsumedSlashCommand {
    pub next_mode: ChatSessionMode,
    pub send_plugin_call: bool,
    pub change_backend: bool,
}

pub fn parse_slash_command(input: &str) -> Option<SlashCommand> {
    match input.trim().to_ascii_lowercase().as_str() {
        "/plan" => Some(SlashCommand::Plan),
        "/build" => Some(SlashCommand::Build),
        "/agents" => Some(SlashCommand::Agents),
        _ => None,
    }
}

pub fn apply_slash_command(current: ChatSessionMode, command: SlashCommand) -> ChatSessionMode {
    match command {
        SlashCommand::Plan => ChatSessionMode::Plan,
        SlashCommand::Build => ChatSessionMode::Build,
        SlashCommand::Agents => match current {
            ChatSessionMode::Plan => ChatSessionMode::Build,
            ChatSessionMode::Build => ChatSessionMode::Plan,
        },
    }
}

pub fn consume_slash_command(
    input: &str,
    current: ChatSessionMode,
) -> Option<ConsumedSlashCommand> {
    let command = parse_slash_command(input)?;
    Some(ConsumedSlashCommand {
        next_mode: apply_slash_command(current, command),
        send_plugin_call: false,
        change_backend: false,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ChatSessionMode, ConsumedSlashCommand, SlashCommand, apply_slash_command,
        consume_slash_command, parse_slash_command,
    };

    #[test]
    fn parses_mode_slash_commands_case_insensitively() {
        assert_eq!(parse_slash_command("/plan"), Some(SlashCommand::Plan));
        assert_eq!(parse_slash_command(" /BUILD "), Some(SlashCommand::Build));
        assert_eq!(parse_slash_command("/Agents"), Some(SlashCommand::Agents));
        assert_eq!(parse_slash_command("/agent"), None);
    }

    #[test]
    fn agents_toggles_only_native_session_modes() {
        assert_eq!(
            apply_slash_command(ChatSessionMode::Plan, SlashCommand::Agents),
            ChatSessionMode::Build
        );
        assert_eq!(
            apply_slash_command(ChatSessionMode::Build, SlashCommand::Agents),
            ChatSessionMode::Plan
        );
    }

    #[test]
    fn slash_commands_are_consumed_without_plugin_call() {
        assert_eq!(
            consume_slash_command("/plan", ChatSessionMode::Build),
            Some(ConsumedSlashCommand {
                next_mode: ChatSessionMode::Plan,
                send_plugin_call: false,
                change_backend: false,
            })
        );
        assert_eq!(
            consume_slash_command("/build", ChatSessionMode::Plan),
            Some(ConsumedSlashCommand {
                next_mode: ChatSessionMode::Build,
                send_plugin_call: false,
                change_backend: false,
            })
        );
        assert_eq!(
            consume_slash_command("/unknown", ChatSessionMode::Plan),
            None
        );
    }

    #[test]
    fn slash_commands_preserve_backend_mode() {
        let first = consume_slash_command("/agents", ChatSessionMode::Plan).unwrap();
        assert_eq!(first.next_mode, ChatSessionMode::Build);
        assert!(!first.send_plugin_call);
        assert!(!first.change_backend);

        let second = consume_slash_command("/agents", first.next_mode).unwrap();
        assert_eq!(second.next_mode, ChatSessionMode::Plan);
        assert!(!second.send_plugin_call);
        assert!(!second.change_backend);
    }
}

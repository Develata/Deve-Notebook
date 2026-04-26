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

#[cfg(test)]
mod tests {
    use super::{ChatSessionMode, SlashCommand, apply_slash_command, parse_slash_command};

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
}

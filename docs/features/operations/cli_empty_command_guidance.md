# cli_empty_command_guidance.md - CLI 空命令提示链

## Metadata

- `Flow ID`: `flow.cli.empty-command-guidance`
- `Domain`: `commands`
- `Related Feature Chapters`: `docs/features/12_commands.md`
- `Related Acceptance Cases`: `CMD-001`

## Operations

### `op.cli.guidance.run-without-command`

- `Name`: `Run CLI Without Subcommand`
- `Surface`: `cli`
- `Trigger`: run bare `deve`
- `Preconditions`: CLI binary is available
- `Immediate Result`: typed command envelope resolves to `None`
- `Application Entry`: `apps/cli/src/main.rs`, `apps/cli/src/dispatch.rs`

### `op.cli.guidance.observe-help-hint`

- `Name`: `Observe Help Hint`
- `Surface`: `terminal-output`
- `Trigger`: dispatcher receives no subcommand
- `Preconditions`: runtime reached `dispatch::run(None)`
- `Immediate Result`: user sees the guidance to use `--help`
- `Application Entry`: `apps/cli/src/dispatch.rs`

## Response Flow

1. User runs the CLI without a subcommand.
2. Instruction interface still parses the call into a typed empty envelope.
3. Flow coordination follows the no-command branch instead of guessing a default action.
4. Execution domain remains `commands`, because guidance is emitted by the control surface boundary.

## Notes

- The current baseline models fail-safe guidance, not implicit default execution.
- Main objects: `cli::command`, `command::selection`, `help::surface`.

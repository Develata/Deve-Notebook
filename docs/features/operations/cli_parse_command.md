# cli_parse_command.md - CLI 命令解析链

## Metadata

- `Flow ID`: `flow.cli.parse-command`
- `Domain`: `commands`
- `Related Feature Chapters`: `docs/features/12_commands.md`
- `Related Acceptance Cases`: `CMD-001`, `CMD-006`, `CMD-007`, `CMD-008`, `CMD-009`

## Operations

### `op.cli.parse.read-argv`

- `Name`: `Read CLI Argv`
- `Surface`: `cli`
- `Trigger`: run `deve ...`
- `Preconditions`: CLI binary is available
- `Immediate Result`: raw argv enters the typed parser
- `Application Entry`: `apps/cli/src/main.rs`

### `op.cli.parse.select-subcommand`

- `Name`: `Select Typed Subcommand`
- `Surface`: `cli`
- `Trigger`: clap finishes argument parsing
- `Preconditions`: argv shape is valid for the declared CLI schema
- `Immediate Result`: request becomes `Option<Commands>`
- `Application Entry`: `apps/cli/src/main.rs`

### `op.cli.parse.observe-typed-command`

- `Name`: `Observe Typed Command Envelope`
- `Surface`: `cli`
- `Trigger`: parsed args are ready for runtime entry
- `Preconditions`: parser completed without clap-level failure
- `Immediate Result`: command envelope is ready for config loading or no-command guidance
- `Application Entry`: `apps/cli/src/main.rs`, `apps/cli/src/dispatch.rs`

## Response Flow

1. User runs `deve` with a command or option set.
2. Instruction interface parses argv into the declared `Commands` schema.
3. Flow coordination produces a typed command envelope instead of ad-hoc string routing.
4. Execution domain remains `commands`, because parse contracts are part of the control surface boundary.

## Notes

- This flow stops at typed command selection; it does not yet execute command modules.
- Main objects: `cli::argv`, `cli::command`, `command::selection`.

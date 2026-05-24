# cli_control_commands.md - CLI 控制面命令链

## Metadata

- `Flow ID`: `flow.cli.control-commands`
- `Domain`: `commands`
- `Related Feature Chapters`: `docs/features/12_commands.md`
- `Related Acceptance Cases`: `CMD-001`, `CMD-002`, `CMD-003`, `CMD-004`
- `Summary-Only`: `yes`

## Operations

### `op.cli.control.invoke-command`

- `Name`: `Invoke CLI Command`
- `Surface`: `cli`
- `Trigger`: run `deve <command>`
- `Preconditions`: CLI binary is available
- `Immediate Result`: command is parsed into a typed command variant
- `Application Entry`: `apps/cli/src/main.rs`, `apps/cli/src/dispatch.rs`

### `op.cli.control.execute-runtime`

- `Name`: `Execute CLI Runtime Command`
- `Surface`: `cli`
- `Trigger`: command dispatcher receives a supported command
- `Preconditions`: config and repo paths are resolved
- `Immediate Result`: command delegates to its module implementation
- `Application Entry`: `apps/cli/src/dispatch.rs`, `apps/cli/src/commands/`

### `op.cli.control.inspect-help`

- `Name`: `Inspect CLI Help`
- `Surface`: `cli`
- `Trigger`: run `deve <command> --help`
- `Preconditions`: command parser is available
- `Immediate Result`: command surface is discoverable without mutating state
- `Application Entry`: `apps/cli/src/main.rs`

## Response Flow

1. User invokes a CLI command or help surface.
2. Instruction interface parses args into `Commands`.
3. Flow coordination dispatches to command modules.
4. Execution domains are ledger, sync, source control, protocol, and config/runtime tooling.

## Notes

- This file is a summary flow. Use the split flows as the authoritative read path for implementation work.
- This flow models the CLI as a first-class control surface, not a debug-only entry.
- Parse, help, empty-command guidance, and runtime handoff are now modeled separately in `cli_parse_command.md`, `cli_help_surface.md`, `cli_empty_command_guidance.md`, and `cli_runtime_handoff.md`.
- Specific command families are modeled in `cli_projection_workspace_indexing.md`, `cli_server_runtime.md`, `cli_export_inspect.md`, and `cli_repair_admin.md`.
- Main objects: `cli::command`, `repo::scope`, `config::runtime`.

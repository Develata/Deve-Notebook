# cli_help_surface.md - CLI 帮助面链

## Metadata

- `Flow ID`: `flow.cli.help-surface`
- `Domain`: `commands`
- `Related Feature Chapters`: `docs/features/12_commands.md`
- `Related Acceptance Cases`: `CMD-001`, `CMD-006`, `CMD-007`, `CMD-009`

## Operations

### `op.cli.help.request-root`

- `Name`: `Request Root Help`
- `Surface`: `cli`
- `Trigger`: run `deve --help`
- `Preconditions`: CLI parser is available
- `Immediate Result`: root command surface becomes inspectable without state mutation
- `Application Entry`: `apps/cli/src/main.rs`

### `op.cli.help.request-command`

- `Name`: `Request Command Help`
- `Surface`: `cli`
- `Trigger`: run `deve <command> --help`
- `Preconditions`: target subcommand is declared in the CLI schema
- `Immediate Result`: command-specific options and usage become inspectable
- `Application Entry`: `apps/cli/src/main.rs`

### `op.cli.help.scan-surface`

- `Name`: `Scan Help Surface`
- `Surface`: `terminal-output`
- `Trigger`: clap renders help text
- `Preconditions`: help request matched a valid CLI surface
- `Immediate Result`: user can inspect supported commands and options before execution
- `Application Entry`: `apps/cli/src/main.rs`

## Response Flow

1. User requests root help or command help.
2. Instruction interface routes the request into clap help rendering instead of runtime execution.
3. Flow coordination keeps the request on the discoverability path.
4. Execution domain remains `commands`, because the help contract belongs to the CLI surface itself.

## Notes

- This flow is intentionally non-mutating.
- Main objects: `cli::command`, `cli::option`, `help::surface`.

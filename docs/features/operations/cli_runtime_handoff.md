# cli_runtime_handoff.md - CLI 运行时移交链

## Metadata

- `Flow ID`: `flow.cli.runtime-handoff`
- `Domain`: `commands`
- `Related Feature Chapters`: `docs/features/12_commands.md`
- `Related Acceptance Cases`: `CMD-001`, `CMD-006`, `CMD-007`, `CMD-008`, `CMD-009`

## Operations

### `op.cli.runtime.load-config`

- `Name`: `Load Runtime Config`
- `Surface`: `cli`
- `Trigger`: typed command envelope is accepted for execution
- `Preconditions`: parser completed successfully
- `Immediate Result`: ledger path, vault path, and profile are materialized for dispatch
- `Application Entry`: `apps/cli/src/main.rs`

### `op.cli.runtime.handoff-dispatch`

- `Name`: `Handoff To Dispatcher`
- `Surface`: `cli`
- `Trigger`: config and paths are ready
- `Preconditions`: runtime bootstrap completed
- `Immediate Result`: dispatcher receives `Some(Commands::...)`
- `Application Entry`: `apps/cli/src/main.rs`, `apps/cli/src/dispatch.rs`

### `op.cli.runtime.enter-command-module`

- `Name`: `Enter Command Module`
- `Surface`: `cli`
- `Trigger`: dispatcher matches a supported command variant
- `Preconditions`: selected command variant is implemented
- `Immediate Result`: control passes into the concrete command module family
- `Application Entry`: `apps/cli/src/dispatch.rs`, `apps/cli/src/commands/`

## Response Flow

1. Runtime bootstrap loads checked config and path state.
2. Instruction interface hands the typed command into the dispatcher.
3. Flow coordination selects the concrete command module family.
4. Execution domains then continue in the command-family flows such as vault, server, export, or repair/admin.

## Notes

- This flow is the bridge between typed CLI control and the already-modeled command families.
- Main objects: `config::runtime`, `command::handoff`, `cli::command`.

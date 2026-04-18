# cli_vault_indexing.md - CLI Vault 索引链

## Metadata

- `Flow ID`: `flow.cli.vault-indexing`
- `Domain`: `commands`
- `Related Feature Chapters`: `docs/features/12_commands.md`
- `Related Acceptance Cases`: `CMD-001`, `CMD-006`

## Operations

### `op.cli.vault.choose-init-path`

- `Name`: `Choose Init Path`
- `Surface`: `cli`
- `Trigger`: run `deve init --path <path>` or `deve init`
- `Preconditions`: CLI binary is available and config loads
- `Immediate Result`: vault path is resolved for initialization
- `Application Entry`: `apps/cli/src/main.rs`, `apps/cli/src/dispatch.rs`

### `op.cli.vault.run-scan`

- `Name`: `Run Vault Scan`
- `Surface`: `cli`
- `Trigger`: run `deve scan`
- `Preconditions`: vault and ledger paths are configured
- `Immediate Result`: scan command enters indexing flow
- `Application Entry`: `apps/cli/src/commands/scan.rs`

### `op.cli.vault.watch-dry-run`

- `Name`: `Watch Vault Dry Run`
- `Surface`: `cli`
- `Trigger`: run `deve watch --dry-run`
- `Preconditions`: vault path is readable
- `Immediate Result`: watcher validates planned reactions without writing changes
- `Application Entry`: `apps/cli/src/commands/watch.rs`

## Response Flow

1. User selects an init, scan, or watch option from the CLI.
2. Instruction interface parses `Commands::{Init, Scan, Watch}`.
3. Flow coordination delegates to the matching command module.
4. Execution domains are config, ledger, tree projection, and filesystem watcher.

## Notes

- This flow covers vault/index lifecycle commands, not long-running server runtime.
- Main objects: `vault::path`, `ledger::snapshot`, `tree::projection`, `cli::option`.

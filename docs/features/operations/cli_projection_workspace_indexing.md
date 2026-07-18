# cli_projection_workspace_indexing.md - CLI Projection Workspace 索引链

## Metadata

- `Flow ID`: `flow.cli.projection-workspace-indexing`
- `Domain`: `commands`
- `Related Feature Chapters`: `docs/features/12_commands.md`
- `Related Acceptance Cases`: `CMD-001`, `CMD-006`

## Operations

### `op.cli.projection.choose-base`

- `Name`: `Choose Projection Base`
- `Surface`: `cli`
- `Trigger`: run `deve init --path <path> --repo <name> --projection-base <path>`
- `Preconditions`: CLI binary is available and config loads
- `Immediate Result`: ledger path is initialized and a host-local Projection Locator is written
- `Application Entry`: `apps/cli/src/main.rs`, `apps/cli/src/dispatch.rs`

### `op.cli.projection.run-scan`

- `Name`: `Run Projection Workspace Scan`
- `Surface`: `cli`
- `Trigger`: run `deve scan`
- `Preconditions`: ledger path is configured and every writable repo has a Projection Locator
- `Immediate Result`: scan command enters indexing flow
- `Application Entry`: `apps/cli/src/commands/scan.rs`

### `op.cli.projection.watch-dry-run`

- `Name`: `Watch Projection Workspace Dry Run`
- `Surface`: `cli`
- `Trigger`: run `deve watch --dry-run`
- `Preconditions`: repo Projection Locators resolve to workspace roots
- `Immediate Result`: watcher validates planned reactions without writing changes
- `Runtime Variant`: without `--dry-run`, `deve watch` owns one non-Clone watcher handle per selected healthy local repo and observes typed worker state
- `Runtime Failure Result`: any terminal worker failure closes all handles in reverse order and exits non-zero; primary failure is not hidden by cleanup errors
- `Shutdown Result`: each handle stops and joins its producer, waits for the synchronous dispatch cut, discards queued hints, performs one exact-root final reconcile and emits at most one typed refresh before the worker join returns
- `Application Entry`: `apps/cli/src/commands/watch.rs`, `crates/core/src/sync/watcher/`

## Response Flow

1. User selects an init, scan, watch dry-run, or standalone watch option from the CLI.
2. Instruction interface parses `Commands::{Init, Scan, Watch}`.
3. Flow coordination delegates to the matching command module.
4. Execution domains are config, Projection Locator runtime, ledger, tree projection, and owned filesystem watcher handles; the CLI owns lifecycle but not ledger authority shortcuts.

## Notes

- This flow covers Projection Workspace / locator lifecycle commands, not long-running server runtime.
- Shutdown `drain` is final-state reconciliation, never raw-event replay; a final scan error is returned after backend/thread cleanup and never rewrites Ledger authority.
- Main objects: `projection::locator`, `projection::workspace`, `ledger::snapshot`, `tree::projection`, `cli::option`.

# cli_repair_admin.md - CLI 修复与管理链

## Metadata

- `Flow ID`: `flow.cli.repair-admin`
- `Domain`: `commands`
- `Related Feature Chapters`: `docs/features/12_commands.md`, `docs/features/06_repository.md`
- `Related Acceptance Cases`: `CMD-001`, `CMD-009`, `REPO-FEAT-03`

## Operations

### `op.cli.admin.verify-p2p`

- `Name`: `Verify P2P Logic`
- `Surface`: `cli`
- `Trigger`: run `deve verify-p2p`
- `Preconditions`: config loads successfully
- `Immediate Result`: P2P simulation validates sync assumptions
- `Application Entry`: `apps/cli/src/commands/verify_p2p.rs`

### `op.cli.admin.seed-peer`

- `Name`: `Seed Peer Data`
- `Surface`: `cli`
- `Trigger`: run `deve seed --peer <peer> [--repo <repo>]`
- `Preconditions`: peer id is provided and repo scope can be resolved
- `Immediate Result`: seed command prepares shadow repo data
- `Application Entry`: `apps/cli/src/commands/seed.rs`

### `op.cli.admin.check-node`

- `Name`: `Check Node Consistency`
- `Surface`: `cli`
- `Trigger`: run `deve node-check [--repair | --projection] [--repo <repo>]`
- `Preconditions`: ledger path is readable
- `Immediate Result`: consistency check runs, optionally enters repair mode, or performs read-only projection authority diagnostics
- `Application Entry`: `apps/cli/src/commands/node_check.rs`

### `op.cli.admin.recover-repo`

- `Name`: `Recover Repo Files`
- `Surface`: `cli`
- `Trigger`: run `deve recover [--repo <repo>]`
- `Preconditions`: ledger data is available for recovery
- `Immediate Result`: recovery flow attempts to restore vault files
- `Application Entry`: `apps/cli/src/commands/recover.rs`

### `op.cli.admin.repair-paths`

- `Name`: `Repair Known Local Corruption`
- `Surface`: `cli`
- `Trigger`: run `deve repair --backup <path> --path <entry> --rebuild-projection`
- `Preconditions`: backup and target paths are readable
- `Immediate Result`: repair strategies run against selected corruption classes
- `Application Entry`: `apps/cli/src/commands/repair/`

## Response Flow

1. User chooses an admin, repair, or P2P maintenance command.
2. Instruction interface parses repair/admin command variants.
3. Flow coordination resolves repo arguments and repair strategy order.
4. Execution domains are ledger, sync, tree projection, source control, and filesystem.

## Notes

- These commands are operational control surfaces, not ordinary document editing flows.
- Main objects: `repo::scope`, `repair::plan`, `tree::projection`, `cli::option`.

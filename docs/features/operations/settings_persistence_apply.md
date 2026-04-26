# settings_persistence_apply.md - Settings 持久化与应用链

## Metadata

- `Flow ID`: `flow.settings.persistence-apply`
- `Domain`: `settings`
- `Related Feature Chapters`: `docs/features/13_settings.md`
- `Related Acceptance Cases`: `SET-001`, `SET-002`, `SET-003`, `SET-004`

## Operations

### `op.settings.persist.write-file`

- `Name`: `Write Settings File Value`
- `Surface`: `editor-or-cli`
- `Trigger`: create or edit `config.toml`, or run `deve config set <key> <value>`
- `Preconditions`: target key is valid in the config schema
- `Immediate Result`: file-backed config value is ready for load
- `Application Entry`: `crates/core/src/config.rs`, `apps/cli/src/commands/init.rs`, `apps/cli/src/commands/config.rs`

### `op.settings.persist.apply-runtime`

- `Name`: `Apply Persisted Config To Runtime`
- `Surface`: `restart-or-boot`
- `Trigger`: CLI/server restarts or reload path reads config
- `Preconditions`: config parses successfully
- `Immediate Result`: effective runtime profile reflects persisted config after safety fallback rules
- `Application Entry`: `Config::load_checked`, `apps/cli/src/main.rs`

### `op.settings.persist.inspect-effective`

- `Name`: `Inspect Effective Setting State`
- `Surface`: `cli-or-log`
- `Trigger`: operator inspects current effective config/profile
- `Preconditions`: runtime has completed config load
- `Immediate Result`: effective setting source and active values are observable
- `Application Entry`: `crates/core/src/config.rs`, `apps/cli/src/server/mod.rs`

## Response Flow

1. User or operator writes config and applies it to runtime.
2. Instruction interface is file-backed config loading and runtime startup.
3. Flow coordination validates, loads, and exposes the effective result.
4. Execution domains are config runtime and server startup.

## Notes

- Persistence/apply is the boundary where settings become authoritative runtime input.
- `ai.mode = "trusted-cli"` is persisted as a requested value, but the effective runtime mode falls
  back to `native` unless trusted-cli policy conditions are satisfied.
- Current runtime persistence is `config.toml` only; any separate settings-backed file or server-backed Settings API is future work.
- Main objects: `settings::file`, `config::apply`, `runtime::profile`.

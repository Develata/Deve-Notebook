# settings_file_config.md - Settings 配置文件链

## Metadata

- `Flow ID`: `flow.settings.file-config`
- `Domain`: `settings`
- `Related Feature Chapters`: `docs/features/13_settings.md`
- `Related Acceptance Cases`: `SET-002`, `SET-004`

## Operations

### `op.settings.file.create-default`

- `Name`: `Create Default Config File`
- `Surface`: `cli`
- `Trigger`: run `deve init --path <path>`
- `Preconditions`: target `config.toml` does not exist
- `Immediate Result`: default config file is written
- `Application Entry`: `apps/cli/src/commands/init.rs`

### `op.settings.file.edit-value`

- `Name`: `Edit Config File Value`
- `Surface`: `editor-or-script`
- `Trigger`: edit `config.toml` or planned `settings.toml`
- `Preconditions`: setting key is supported by the config schema
- `Immediate Result`: file-backed value is ready for next load
- `Application Entry`: `crates/core/src/config.rs`

### `op.settings.file.restart-apply`

- `Name`: `Restart To Apply File Config`
- `Surface`: `cli-or-process-manager`
- `Trigger`: restart CLI/server after editing the file
- `Preconditions`: config file parses successfully
- `Immediate Result`: runtime config reflects file value
- `Application Entry`: `Config::load_checked`

## Response Flow

1. User creates or edits the settings/config file.
2. Instruction interface is file-backed config loading.
3. Flow coordination validates schema and applies supported fields.
4. Execution domains are config runtime, commands, and server startup.

## Notes

- Current implementation reads `config.toml`; `settings.toml` remains the user-facing naming target if later introduced.
- Main objects: `settings::file`, `config::runtime`, `runtime::profile`.

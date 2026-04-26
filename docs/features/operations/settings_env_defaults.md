# settings_env_defaults.md - Settings 环境默认值链

## Metadata

- `Flow ID`: `flow.settings.env-defaults`
- `Domain`: `settings`
- `Related Feature Chapters`: `docs/features/13_settings.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `SET-001`

## Operations

### `op.settings.env.leave-unset`

- `Name`: `Leave Environment Unset`
- `Surface`: `shell`
- `Trigger`: start without `DEVE_PROFILE` or related `DEVE_*` overrides
- `Preconditions`: no overriding environment variable is present
- `Immediate Result`: config loader falls back to default values
- `Application Entry`: `crates/core/src/config.rs`

### `op.settings.env.set-override`

- `Name`: `Set Environment Override`
- `Surface`: `shell`
- `Trigger`: set `DEVE_PROFILE`, `DEVE_LEDGER_DIR`, or `DEVE_VAULT_PATH`
- `Preconditions`: environment variable value is parseable
- `Immediate Result`: environment source overrides file/default settings
- `Application Entry`: `crates/core/src/config.rs`

### `op.settings.env.inspect-effective`

- `Name`: `Inspect Effective Environment Config`
- `Surface`: `cli-or-log`
- `Trigger`: inspect startup profile or config output
- `Preconditions`: config has loaded successfully
- `Immediate Result`: operator can see effective default or override
- `Application Entry`: `apps/cli/src/main.rs`, `crates/core/src/config.rs`

## Response Flow

1. User starts the process with or without `DEVE_*` overrides.
2. Instruction interface is config bootstrap during CLI/server startup.
3. Flow coordination merges environment, `.env`, config file, and defaults.
4. Execution domains are config runtime, commands, and server runtime.

## Notes

- Environment defaults are runtime input, not UI preference state.
- Flat runtime keys keep single underscores, for example `DEVE_LEDGER_DIR`; nested keys use double underscores.
- Main objects: `env::override`, `config::runtime`, `runtime::profile`.

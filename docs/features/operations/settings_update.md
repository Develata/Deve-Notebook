# settings_update.md - Settings 更新链

## Metadata

- `Flow ID`: `flow.settings.update`
- `Domain`: `settings`
- `Related Feature Chapters`: `docs/features/13_settings.md`, `docs/features/12_commands.md`
- `Related Acceptance Cases`: `SET-001`, `SET-002`, `CMD-001`

## Operations

### `op.settings.open-surface`

- `Name`: `Open Settings Surface`
- `Surface`: `settings-or-command-palette`
- `Trigger`: open settings from UI or command surface
- `Preconditions`: application shell is loaded
- `Immediate Result`: settings groups are visible
- `Application Entry`: `apps/web/src/components/settings.rs`, `apps/web/src/components/settings_sections.rs`

### `op.settings.change-value`

- `Name`: `Change Setting Value`
- `Surface`: `settings-panel-or-cli`
- `Trigger`: select theme, language, panel state, or config value
- `Preconditions`: target setting is implemented and not marked future-only
- `Immediate Result`: runtime state or config draft updates
- `Application Entry`: `apps/web/src/components/settings_sections.rs`, `apps/cli/src/dispatch.rs`

### `op.settings.observe-feedback`

- `Name`: `Observe Setting Feedback`
- `Surface`: `workspace-shell`
- `Trigger`: setting update completes
- `Preconditions`: updated setting has visible or inspectable effect
- `Immediate Result`: user sees immediate UI feedback or CLI config output
- `Application Entry`: `apps/web/src/components/settings.rs`, `crates/core/src/config.rs`

## Response Flow

1. User opens settings or invokes a config command.
2. Instruction interface translates the change into a settings/config update.
3. Flow coordination applies the supported setting and keeps reserved settings clearly separated.
4. Execution domains are config, UI shell runtime, i18n, and command/control.

## Notes

- Settings must not bypass authority state or mutate ledger truth directly.
- Main objects: `config::runtime`, `ui::preference`, `locale::selection`.

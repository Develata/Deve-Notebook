# settings_value_mutation.md - Settings 值变更链

## Metadata

- `Flow ID`: `flow.settings.value-mutation`
- `Domain`: `settings`
- `Related Feature Chapters`: `docs/features/13_settings.md`, `docs/features/11_i18n.md`
- `Related Acceptance Cases`: `SET-005`, `SET-002`, `CMD-001`

## Operations

### `op.settings.mutation.change-ui`

- `Name`: `Change UI Setting Value`
- `Surface`: `settings-modal`
- `Trigger`: click theme, language, or panel-related choice
- `Preconditions`: target setting is currently supported
- `Immediate Result`: UI preference draft updates
- `Application Entry`: `apps/web/src/components/settings.rs`, `apps/web/src/components/settings_sections.rs`

### `op.settings.mutation.change-runtime`

- `Name`: `Change Runtime Mode Value`
- `Surface`: `settings-modal`
- `Trigger`: click sync mode or AI backend choice
- `Preconditions`: capability gate and runtime context are available
- `Immediate Result`: runtime-facing setting callback receives the new value
- `Application Entry`: `apps/web/src/components/settings_sections.rs`

### `op.settings.mutation.change-cli`

- `Name`: `Change CLI Config Value`
- `Surface`: `cli`
- `Trigger`: invoke config-related command or edit-backed command path
- `Preconditions`: CLI is available and target key is supported
- `Immediate Result`: runtime config target value is updated or prepared
- `Application Entry`: `apps/cli/src/dispatch.rs`, `crates/core/src/config.rs`

## Response Flow

1. User changes a setting value through UI or CLI.
2. Instruction interface translates the change into a typed setting mutation.
3. Flow coordination routes the mutation to supported runtime/config handlers.
4. Execution domains are settings, i18n, sync, plugin capability, and command/control.

## Notes

- Value mutation is separate from persistence and separate from visible feedback.
- Main objects: `settings::draft`, `ui::preference`, `config::runtime`.

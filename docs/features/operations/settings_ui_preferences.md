# settings_ui_preferences.md - Settings UI 偏好链

## Metadata

- `Flow ID`: `flow.settings.ui-preferences`
- `Domain`: `settings`
- `Related Feature Chapters`: `docs/features/13_settings.md`, `docs/features/11_i18n.md`
- `Related Acceptance Cases`: `SET-005`, `I18N-001`, `I18N-002`

## Operations

### `op.settings.ui.select-language`

- `Name`: `Select UI Language`
- `Surface`: `settings-modal`
- `Trigger`: click English or 中文 in Settings
- `Preconditions`: settings modal is open
- `Immediate Result`: locale signal updates and labels re-render
- `Application Entry`: `apps/web/src/components/settings.rs`

### `op.settings.ui.select-sync-mode`

- `Name`: `Select Sync Mode`
- `Surface`: `settings-modal`
- `Trigger`: click Auto or Manual sync mode
- `Preconditions`: core sync context is available
- `Immediate Result`: sync mode callback receives the selected mode
- `Application Entry`: `apps/web/src/components/settings_sections.rs`

### `op.settings.ui.select-ai-backend`

- `Name`: `Select AI Backend`
- `Surface`: `settings-modal`
- `Trigger`: click Native or Trusted CLI backend
- `Preconditions`: backend capability check has completed
- `Immediate Result`: AI mode updates or disabled state explains why it cannot
- `Application Entry`: `apps/web/src/components/settings_sections.rs`

## Response Flow

1. User changes a visible settings control.
2. Instruction interface is the settings modal callback.
3. Flow coordination updates local UI state or delegates to core callbacks.
4. Execution domains are settings, i18n, sync, and plugin/agent capability.

## Notes

- UI preferences must not directly mutate ledger authority state.
- Main objects: `ui::preference`, `locale::selection`, `ai::backend-capability`.

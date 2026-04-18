# i18n_locale_selection.md - I18n 语言选择链

## Metadata

- `Flow ID`: `flow.i18n.locale-selection`
- `Domain`: `i18n`
- `Related Feature Chapters`: `docs/features/11_i18n.md`, `docs/features/13_settings.md`
- `Related Acceptance Cases`: `I18N-002`, `I18N-003`, `SET-005`

## Operations

### `op.i18n.locale.use-browser`

- `Name`: `Use Browser Locale`
- `Surface`: `browser`
- `Trigger`: app starts with locale set to auto
- `Preconditions`: browser language is available
- `Immediate Result`: locale resolver selects the preferred supported locale or fallback
- `Application Entry`: `apps/web/src/app.rs`, `apps/web/src/i18n/mod.rs`

### `op.i18n.locale.select-explicit`

- `Name`: `Select Explicit Locale`
- `Surface`: `settings-modal`
- `Trigger`: click English or 中文 in Settings
- `Preconditions`: settings modal is open
- `Immediate Result`: locale signal changes and UI text re-renders
- `Application Entry`: `apps/web/src/components/settings.rs`

### `op.i18n.locale.fallback-missing`

- `Name`: `Fallback Missing Locale`
- `Surface`: `i18n-runtime`
- `Trigger`: unsupported locale or missing key is requested
- `Preconditions`: fallback locale is available
- `Immediate Result`: UI falls back to stable English text
- `Application Entry`: `apps/web/src/i18n/`

## Response Flow

1. User or browser provides a locale preference.
2. Instruction interface resolves browser/settings selection into `Locale`.
3. Flow coordination applies fallback rules before rendering text.
4. Execution domains are settings and i18n catalog.

## Notes

- Locale choice is presentation state; it must not mutate ledger authority.
- Main objects: `locale::selection`, `i18n::catalog`, `ui::preference`.

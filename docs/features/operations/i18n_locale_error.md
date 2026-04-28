# i18n_locale_error.md - Locale 与错误码文案链

## Metadata

- `Flow ID`: `flow.i18n.locale-error`
- `Domain`: `i18n`
- `Related Feature Chapters`: `docs/features/11_i18n.md`, `docs/features/09_auth.md`
- `Related Acceptance Cases`: `I18N-001`, `I18N-002`, `I18N-003`, `I18N-004`, `I18N-005`, `I18N-006`
- `Summary-Only`: `yes`

## Operations

### `op.i18n.select-locale`

- `Name`: `Select Locale`
- `Surface`: `settings-or-browser`
- `Trigger`: choose locale or use browser language when locale is auto
- `Preconditions`: i18n facade is loaded
- `Immediate Result`: UI text resolves through locale facade
- `Application Entry`: `apps/web/src/i18n/`, `apps/web/src/components/settings_sections.rs`

### `op.i18n.receive-error-code`

- `Name`: `Receive Error Code`
- `Surface`: `api-or-websocket`
- `Trigger`: backend returns auth, source-control, or sync error
- `Preconditions`: error is represented as a structured code
- `Immediate Result`: frontend maps code to localized visible text
- `Application Entry`: `apps/web/src/i18n/server_error.rs`, `crates/core/src/protocol/`

### `op.i18n.observe-format`

- `Name`: `Observe Localized Format`
- `Surface`: `workspace-shell`
- `Trigger`: visible date, time, number, or status text renders
- `Preconditions`: locale is selected or detected
- `Immediate Result`: visible text follows locale rules or falls back explicitly
- `Application Entry`: `apps/web/src/utils/time.rs`, `apps/web/src/i18n/`

## Response Flow

1. User selects or implicitly receives a locale.
2. Instruction interface resolves locale and error-code lookup through i18n facade.
3. Flow coordination keeps backend protocol stable as structured codes, not natural-language payloads.
4. Execution domains are i18n catalog, protocol, settings, and formatting utilities.

## Notes

- This file is a summary flow. Use the split locale / error flows as the authoritative implementation read path.
- Missing-key behavior is compile-time constrained by the current Rust `t::*` facade; runtime fallback applies to unsupported locale tags. If external resource files are introduced later, missing-key fallback must be implemented there.
- Main objects: `locale::selection`, `i18n::catalog`, `protocol::error-code`.

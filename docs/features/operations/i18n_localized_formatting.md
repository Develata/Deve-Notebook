# i18n_localized_formatting.md - I18n 本地化格式链

## Metadata

- `Flow ID`: `flow.i18n.localized-formatting`
- `Domain`: `i18n`
- `Related Feature Chapters`: `docs/features/11_i18n.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `I18N-005`, `TECH-001`

## Operations

### `op.i18n.format.switch-locale`

- `Name`: `Switch Formatting Locale`
- `Surface`: `settings-modal`
- `Trigger`: user switches locale while time or number text is visible
- `Preconditions`: formatted value is present on screen
- `Immediate Result`: visible formatted value should update for the new locale
- `Application Entry`: `apps/web/src/i18n/`, `apps/web/src/utils/time.rs`

### `op.i18n.format.observe-relative-time`

- `Name`: `Observe Relative Time Format`
- `Surface`: `workspace-shell`
- `Trigger`: timestamp or activity status renders
- `Preconditions`: timestamp value exists
- `Immediate Result`: user sees locale-aware time text
- `Application Entry`: `apps/web/src/utils/time.rs`

### `op.i18n.format.audit-manual-date`

- `Name`: `Audit Manual Date Formatting`
- `Surface`: `repo-search`
- `Trigger`: search for hand-built date or number formatting
- `Preconditions`: frontend source is readable
- `Immediate Result`: manual formatting candidates are visible for cleanup
- `Application Entry`: `apps/web/src/utils/time.rs`, `apps/web/src/components/`

## Response Flow

1. User switches locale or views localized status text.
2. Instruction interface resolves locale and formatting request.
3. Flow coordination chooses formatting utility rather than manual string assembly.
4. Execution domains are i18n, formatting utilities, and tech-stack constraints.

## Notes

- Manual date/time formatting is guarded by `scripts/check-i18n-formatting-baseline.sh`.
- Main objects: `locale::selection`, `format::localized`, `i18n::catalog`.

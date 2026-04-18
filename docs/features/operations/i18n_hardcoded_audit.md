# i18n_hardcoded_audit.md - I18n 硬编码文案审计链

## Metadata

- `Flow ID`: `flow.i18n.hardcoded-audit`
- `Domain`: `i18n`
- `Related Feature Chapters`: `docs/features/11_i18n.md`
- `Related Acceptance Cases`: `I18N-001`, `I18N-003`

## Operations

### `op.i18n.audit.scan-hardcoded`

- `Name`: `Scan Hardcoded Visible Text`
- `Surface`: `repo-search`
- `Trigger`: run source search for visible hardcoded strings
- `Preconditions`: frontend source is readable
- `Immediate Result`: non-facade visible text candidates are listed
- `Application Entry`: `apps/web/src/components/`, `apps/web/src/i18n/`

### `op.i18n.audit.add-key`

- `Name`: `Add I18n Key`
- `Surface`: `code-edit`
- `Trigger`: replace visible hardcoded text with `t::*` facade call
- `Preconditions`: target module has an i18n namespace
- `Immediate Result`: new text is represented in English and Chinese
- `Application Entry`: `apps/web/src/i18n/`

### `op.i18n.audit.verify-facade`

- `Name`: `Verify I18n Facade Usage`
- `Surface`: `repo-search-or-test`
- `Trigger`: run audit after adding or changing UI text
- `Preconditions`: updated source compiles or is searchable
- `Immediate Result`: visible text is routed through `t::*`
- `Application Entry`: `apps/web/src/i18n/mod.rs`

## Response Flow

1. Maintainer searches for visible text outside the i18n facade.
2. Instruction interface is repo search or review automation.
3. Flow coordination adds missing keys and verifies facade usage.
4. Execution domains are i18n catalog and tech-stack validation.

## Notes

- Audit operations are maintainer-facing, but they protect user-visible text consistency.
- Main objects: `i18n::key-audit`, `i18n::catalog`, `ui::text`.

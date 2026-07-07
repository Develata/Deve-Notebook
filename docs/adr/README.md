# Architecture Decision Records (ADR)

This directory is the project's **decision history slice**: a time-ordered log of
significant architecture decisions, parallel to (not part of) the engineering
blueprint in `docs/plan/`.

## Numbering & naming

- Files are `NNNN-kebab-case-title.md`, `NNNN` a zero-padded sequential id starting at `0001`.
- Ids are never reused; a reversed decision is recorded as a new ADR that marks the old one `Superseded`.

## Status values

`Proposed` · `Accepted` · `Superseded by NNNN` · `Deprecated`

## Template

```markdown
# NNNN. <decision title>

- Status: <Proposed | Accepted | Superseded by NNNN | Deprecated>
- Date: <YYYY-MM-DD>

## Context
<forces, constraints, alternatives considered>

## Decision
<what was chosen and the core rationale>

## Consequences
<resulting trade-offs, follow-ups, what this commits us to>

## References
<plain paths to docs/plan chapters that this decision shaped — NOT plan_ref annotations>
```

## plan_ref boundary (governance)

- An ADR is a **decision log** (time attribute), not an engineering-blueprint clause.
- ADRs **MUST NOT** be referenced by code `plan_ref:` annotations; `plan_ref` targets only `docs/plan/` chapter anchors.
- This is enforced by `scripts/plan-coverage.sh --check-no-adr-plan-ref` (fails if any code `plan_ref` targets `adr/...`).
- ADRs **may cite** plan chapters for context (plain path). Plan chapters **may** note a relevant ADR as decision history, but **MUST NOT** depend on an ADR as a normative implementation clause; code `plan_ref` **MUST NOT** target an ADR.

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](./0001-leptos-over-yew.md) | Leptos over Yew for the frontend | Accepted |
| [0002](./0002-redb-over-sled.md) | redb over sled for embedded storage | Accepted |
| [0003](./0003-self-i18n-over-fluent.md) | Self-built i18n facade over Fluent | Accepted |
| [0004](./0004-tauri-v2-as-native-target.md) | Tauri v2 as the native packaging target | Accepted |
| [0005](./0005-uuid-first-not-path-first.md) | UUID-first identity, not path-first | Accepted |
| [0006](./0006-native-linux-gtk3-first-tag-route.md) | Native Linux GTK3 first-tag route | Proposed |
| [0007](./0007-backup-explicit-merge-authority-route.md) | Backup explicit merge authority route | Proposed |

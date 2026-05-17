# Plugin Runtime Acceptance Closure - 2026-05-17

本报告记录 `PLUG-002` 与 `PLUG-003` 的自动守卫闭合。`docs/plan/` 未修改。

## Scope

- `PLUG-002`: Calculation Runtime remains interface-only and visibly disabled.
- `PLUG-003`: ledger-managed plugin boundaries remain future-hard constraints, with Rhai module imports guarded against traversal and symlink escape.

## Changes

- `scripts/check-ai-baseline.sh` now binds both plugin cases to existing UI reserved-state checks, plan boundary checks, and module resolver sandbox tests.
- No plugin runtime behavior was changed.
- MCP runtime and general code execution remain closed.

## Verification

Ran:

- `bash scripts/check-ai-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`

Results:

- AI/plugin baseline: passed.
- Acceptance bindings after this batch: automated `133`, feature walkthrough `54`, manual `13`, unbound `0`.

## Decision

Plugin runtime acceptance residue is closed for the selected current cases.

Next batch: **Rendering Manual Acceptance Closure**.

First targets:

1. Bind `RENDER-CODE-001`, `RENDER-LINK-002`, `RENDER-NEST-001`, and `RENDER-WHITELIST-001` to existing rendering guards and focused tests where possible.
2. Keep source-first editor authority unchanged.
3. Do not claim complete WYSIWYG, arbitrary HTML, or full virtual rendering.

# Rendering Manual Acceptance Closure - 2026-05-17

本报告记录 `RENDER-CODE-001`、`RENDER-LINK-002`、`RENDER-NEST-001` 与 `RENDER-WHITELIST-001` 的自动守卫闭合。`docs/plan/` 未修改。

## Scope

- `RENDER-CODE-001`: code block toolbar keeps copy/menu controls and empty menu state when no action is registered.
- `RENDER-LINK-002`: rendered external links keep `_blank` plus `noopener noreferrer` safety attributes.
- `RENDER-NEST-001`: nested quote/math decorations remain present and backed by browser smoke evidence.
- `RENDER-WHITELIST-001`: unsupported highlight syntax remains plain text and arbitrary HTML remains filtered.

## Changes

- `scripts/check-rendering-baseline.sh` now binds the four rendering cases to existing renderer tests, CodeMirror adapter guards, CSS/JS depth decorations, and Chrome MCP smoke reports.
- No renderer runtime behavior was changed.
- Source-first editor authority remains unchanged.

## Verification

Ran:

- `bash scripts/check-rendering-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`

Results:

- Rendering baseline: passed.
- Acceptance bindings after this batch: automated `137`, feature walkthrough `54`, manual `9`, unbound `0`.

## Decision

Rendering manual acceptance residue is closed for the selected current cases.

Next batch: **Command Acceptance Boundary Closure**.

First target:

1. Triage `CMD-005` and bind it only to current command behavior. Do not introduce Web Git writer, server-backed Settings API, native process runtime, signing, physical-device, or native authority writes.

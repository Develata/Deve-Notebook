# AI Chat UX Acceptance Closure - 2026-05-17

本报告记录 `AI-001` 与 `AI-004` 的自动守卫闭合。`docs/plan/` 未修改。

## Scope

- `AI-001`: Native AI Chat uses current Markdown context and finishes matching chat placeholders.
- `AI-004`: `/agents` toggles only Native PLAN / BUILD session mode and preserves backend mode.

## Changes

- `scripts/check-ai-baseline.sh` now binds both case ids to existing focused tests and implementation checks.
- No AI runtime behavior was changed.
- Trusted CLI remains default-off and policy-gated.

## Verification

Ran:

- `bash scripts/check-ai-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`

Results:

- AI baseline: passed.
- Acceptance bindings after this batch: automated `131`, feature walkthrough `54`, manual `15`, unbound `0`.

## Decision

AI Chat UX acceptance residue is closed for the selected current cases.

Next batch: **Plugin Runtime Acceptance Closure**.

First targets:

1. Bind `PLUG-002` and `PLUG-003` to existing plugin UI reserved-state and sandbox/security guards.
2. Keep Calculation Runtime interface-only/default-disabled.
3. Do not reopen MCP runtime or add general code execution capability.

# Command Acceptance Boundary Closure - 2026-05-17

本报告记录 `CMD-005` 的自动守卫闭合。`docs/plan/` 未修改。

## Scope

- `CMD-005`: AI slash commands switch only local Native `PLAN` / `BUILD` session mode.
- `/plan`, `/build`, and `/agents` must not switch `native` / `trusted-cli` backend.
- Slash commands must not dispatch plugin calls by themselves.

## Changes

- `scripts/check-ai-baseline.sh` now binds `CMD-005` to the same slash-command guard already used by `AI-004`.
- No chat, command palette, backend, or plugin runtime behavior was changed.

## Verification

Ran:

- `bash scripts/check-ai-baseline.sh`
- `cargo test -p deve_web slash_commands -- --nocapture`
- `bash scripts/check-acceptance-bindings.sh`

Results:

- AI baseline: passed.
- Slash command tests: `4` passed.
- Acceptance bindings after this batch: automated `138`, feature walkthrough `54`, manual `8`, unbound `0`.

## Decision

Command acceptance residue is closed for the selected current case.

Remaining manual cases are documentation-review cases in positioning and terminology. They should stay manual-doc unless a concrete plan/code drift appears.

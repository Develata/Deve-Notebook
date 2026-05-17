# Mainline Gap Rescan After Settings Local Feedback Closure - 2026-05-17

本报告记录 Settings Local Persistence / Feedback Closure 后的主线守卫复扫。`docs/plan/` 未修改。

## Closed Batch

- `docs/report/settings-local-feedback-baseline-2026-05-17.md`
- `docs/report/settings-local-feedback-browser-smoke-2026-05-17.md`

## Guards

Ran:

- `bash scripts/check-settings-local-feedback-baseline.sh`
- `bash scripts/check-cli-settings-baseline.sh`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-architecture-registry.sh`
- `bash scripts/plan-coverage.sh`
- `bash scripts/smoke-runtime-happy-path.sh`
- `bash scripts/smoke-runtime-recovery-path.sh`
- `git diff --check`

Results:

- Settings local feedback baseline: passed.
- CLI settings baseline: passed.
- Source Control baseline: passed.
- Acceptance bindings: automated `117`, feature walkthrough `54`, manual `29`, unbound soft `0`.
- Feature operation paths: ok.
- Architecture registry: `72` flows, active drift `0`.
- Plan coverage: blocking violations `0`, soft warnings `18`.
- Runtime happy/recovery smoke: passed.
- Diff hygiene: passed.

## Decision

Settings Local Persistence / Feedback Closure is closed for current Web/server scope.

Next local mainline batch: **Source Control Command Surface Refresh**.

Rationale:

- It was the remaining local candidate from the prior mainline selection path.
- It has no signing material, target host, physical device, store credential, or native process prerequisite.
- It must stay inside existing Source Control panel, command palette notices, CLI-only Git mirror boundary, and current source-control HTTP/WS gates.
- It must not implement Web Git writer, server-backed Settings API, native process runtime, platform signing, or native authority writes.

## First Targets

1. Re-run Source Control command-surface and baseline tests.
2. Browser-smoke Command Palette Source Control / Git / AI reserved entries and Source Control panel commit surface.
3. Fix only concrete runtime or feedback gaps found by that smoke.
4. Record the browser smoke in `docs/report/` and keep `docs/plan/` unchanged.

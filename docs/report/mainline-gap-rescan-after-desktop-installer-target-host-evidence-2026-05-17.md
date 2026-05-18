# Mainline Gap Rescan After Desktop Installer Target-host Evidence - 2026-05-17

本报告记录 Desktop installer target-host evidence closure 后的主线缺口复扫。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/`.
- Input evidence: `docs/report/desktop-installer-target-host-evidence-refresh-2026-05-17.md`.
- Boundary: Current Web/server + Desktop package/startup/native-session/installer target-host evidence + shell-only non-Desktop platform gates.
- Non-goal: signing、store、physical-device readiness、native authority writes、Android process runtime、Web Git writer、server-backed Settings API。

## Verification

- `bash scripts/check-acceptance-bindings.sh`: automated `149`, feature walkthrough `54`, manual `0`, unbound `0`.
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-architecture-registry.sh`: flows `72`, active drift `0`.
- `bash scripts/check-release-baseline.sh`
- Foundation/network/auth/rendering baselines.
- Storage/repo、source-control、repo file ops、CLI settings、settings local feedback baselines.
- Native process adapter gate、native packaging gate、mobile baseline.
- Runtime smoke: `scripts/smoke-runtime-happy-path.sh`, `scripts/smoke-runtime-recovery-path.sh`.
- AI/search/graph/large-doc baselines.
- Dev-runbook、dev-data-health、diff-color、i18n formatting、i18n hardcoded baselines.
- UI baselines: dashboard refresh、desktop、disconnect、focus、SPA routing、token、z-index.
- `bash scripts/plan-coverage.sh`: blocking violations `0`, dangling `plan_ref` `0`, i18n leaks `0`, soft warnings `27`.
- Diff hygiene: `git diff --check`.

## Findings

- Desktop macOS / Windows installer target-host evidence is green at `04723cef`.
- Installer evidence remains unsigned package-shape smoke; it does not claim signing、store or physical-device readiness.
- Process runtime gate and native authority writes remain closed.
- Android remains shell-only; Android process runtime remains closed.
- No new unblocked Current Web/server `MUST` gap was found.
- Local Linux native packaging gate emitted expected appindicator/EGL warnings and skipped local package smoke by script contract; macOS / Windows target-host evidence remains the Desktop authority.

## Decision

The mainline gap rescan is closed. The next batch should run a full regression gate refresh before selecting another feature or platform target.


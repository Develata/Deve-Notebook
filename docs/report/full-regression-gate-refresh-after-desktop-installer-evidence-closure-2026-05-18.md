# Full Regression Gate Refresh After Desktop Installer Evidence Closure - 2026-05-18

本报告记录 Desktop installer target-host evidence closure 后的 full regression gate。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/`.
- Input evidence: `docs/report/mainline-gap-rescan-after-desktop-installer-target-host-evidence-2026-05-17.md`.
- Boundary: Current Web/server + Desktop package/startup/native-session/installer target-host evidence + shell-only non-Desktop platform gates.
- Non-goal: signing、store、physical-device readiness、native authority writes、Android process runtime、Web Git writer、server-backed Settings API。

## Verification

- `cargo fmt --check`
- `cargo test --locked`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `bash scripts/check-acceptance-bindings.sh`: automated `149`, feature walkthrough `54`, manual `0`, unbound `0`.
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-architecture-registry.sh`: flows `72`, active drift `0`.
- `bash scripts/check-release-baseline.sh`
- `bash scripts/plan-coverage.sh`: blocking violations `0`, dangling `plan_ref` `0`, i18n leaks `0`, soft warnings `27`.
- Domain baselines: foundation, network, auth, rendering, storage/repo, source-control, settings, CLI settings, repo file ops, AI, search, graph, large-doc, dev-runbook, dev-data-health, diff-color, i18n formatting, i18n hardcoded, UI dashboard/desktop/disconnect/focus/SPA routing/token/z-index.
- Native/mobile gates: `scripts/check-native-process-adapter-gate.sh`, `scripts/check-native-packaging-gate.sh`, `scripts/check-mobile-baseline.sh`.
- Runtime smoke: `scripts/smoke-runtime-happy-path.sh`, `scripts/smoke-runtime-recovery-path.sh`.
- Web release build: `scripts/smoke-web-release-build.sh`.
- Runtime release-info smoke: `scripts/smoke-runtime-release-info.sh`，本地无服务时按脚本跳过。
- Diff hygiene: `git diff --check`.

## Findings

- Full regression gate passed without code changes after the Desktop installer evidence report.
- `scripts/smoke-runtime-release-info.sh` skipped because no local service was reachable at `http://127.0.0.1:3001/api/node/role`.
- Native packaging gate emitted expected local Linux appindicator/EGL warnings and finished `ok`; macOS / Windows target-host package and installer smoke remain the Desktop authority.
- No new unblocked Current Web/server `MUST` gap was found during this gate.
- Validation introduced no code or configuration changes; only this report and the active queue summary were updated afterward.

## Decision

The full regression gate is closed. The next batch should select the next small implementation or evidence target from the current plan without opening broad platform runtime scope.

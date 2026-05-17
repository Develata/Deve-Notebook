# Full Regression Gate Refresh After Desktop Native Session Evidence Closure - 2026-05-17

本报告记录 Desktop native-session target-host evidence closure 后的 full regression gate。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/`.
- Input evidence: `docs/report/mainline-gap-rescan-after-desktop-native-session-target-host-evidence-2026-05-17.md`.
- Boundary: Current Web/server + Desktop native-session evidence + shell-only non-Desktop platform gates.
- Non-goal: Android process runtime、native authority writes、signing、store、physical-device readiness、Web Git writer、server-backed Settings API。

## Verification

- `cargo fmt --check`
- `cargo test --locked`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `bash scripts/check-acceptance-bindings.sh`: automated `149`, feature walkthrough `54`, manual `0`, unbound `0`.
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-architecture-registry.sh`: flows `72`, active drift `0`.
- `bash scripts/plan-coverage.sh`: blocking violations `0`, dangling `plan_ref` `0`, i18n leaks `0`, soft warnings `27`.
- Domain baselines: foundation, network, auth, rendering, storage/repo, source-control, settings, CLI settings, repo file ops, AI, search, graph, mobile, release, dev-runbook, dev-data-health, diff-color, i18n, large-doc, UI desktop/dashboard/disconnect/focus/spa-routing/token/z-index.
- Native gates: `scripts/check-native-process-adapter-gate.sh`, `scripts/check-native-packaging-gate.sh`.
- Runtime smoke: `scripts/smoke-runtime-happy-path.sh`, `scripts/smoke-runtime-recovery-path.sh`.
- Web release build: `scripts/smoke-web-release-build.sh`.
- Diff hygiene: `git diff --check`.

## Findings

- Full regression gate passed without code changes.
- `scripts/smoke-runtime-release-info.sh` skipped because no local service was reachable at `http://127.0.0.1:3001/api/node/role`.
- Native packaging gate emitted expected local Linux appindicator/EGL warnings and finished `ok`; macOS / Windows target-host package smoke remains the Desktop native-session authority.
- No new unblocked Current Web/server `MUST` gap was found during this gate.
- Working tree was clean after validation.

## Decision

The full regression gate is closed. The next batch should select the next implementation or evidence target from the current plan without opening broad platform runtime scope.


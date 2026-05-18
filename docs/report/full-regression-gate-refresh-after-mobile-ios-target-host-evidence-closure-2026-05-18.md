# Full Regression Gate Refresh After Mobile iOS Target-host Evidence Closure - 2026-05-18

本报告记录 Mobile iOS shell-only target-host evidence closure 后的 full regression gate。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/`.
- Input evidence: `docs/report/mainline-gap-rescan-after-mobile-ios-target-host-evidence-refresh-2026-05-18.md`.
- Boundary: Current Web/server + Desktop package/startup/native-session/installer target-host evidence + Android shell-only package/install/startup target-host evidence + iOS shell-only package/install/startup target-host evidence.
- Non-goal: signing、store、physical-device readiness、native authority writes、Mobile process runtime、Android process runtime、Web Git writer、server-backed Settings API。

## Verification

- `cargo fmt --check`
- `cargo test --locked`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `bash scripts/check-acceptance-bindings.sh`: automated `149`, feature walkthrough `54`, manual `0`, unbound `0`.
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-architecture-registry.sh`: flows `72`, active drift `0`.
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-release-audit-gate.sh`: local `cargo-audit` unavailable and skipped by diagnostic policy; npm audit found `0` high vulnerabilities.
- `bash scripts/plan-coverage.sh`: blocking violations `0`, dangling `plan_ref` `0`, i18n leaks `0`, soft warnings `27`.
- Domain baselines: foundation, network, auth, rendering, storage/repo, source-control, repo file ops, CLI settings, settings local feedback, AI, search, graph, large-doc, dev-runbook, dev-data-health, diff-color, i18n formatting, i18n hardcoded.
- UI baselines: dashboard refresh, desktop, disconnect, focus, SPA routing, token, z-index.
- Native/mobile gates: `scripts/check-native-process-adapter-gate.sh`, `scripts/check-native-packaging-gate.sh`, `scripts/check-mobile-baseline.sh`.
- Target-host evidence validator: iOS evidence + Android evidence passed `scripts/check-native-target-host-evidence.sh`.
- Runtime smoke: `scripts/smoke-runtime-happy-path.sh`, `scripts/smoke-runtime-recovery-path.sh`.
- Runtime release-info smoke: `scripts/smoke-runtime-release-info.sh`，本地无服务时按脚本跳过。
- Web release build: `scripts/smoke-web-release-build.sh`.
- Docker release smoke: `scripts/smoke-docker-release.sh`.
- Diff hygiene: `git diff --check`.

## Findings

- Full regression gate passed after Desktop、Android、iOS target-host evidence closure.
- Docker release smoke passed with cached release image build and production runtime smoke.
- Native packaging gate emitted expected local Linux appindicator/EGL warnings and finished `ok`; target-host evidence remains the Desktop/Android/iOS package authority.
- Runtime release-info smoke skipped because no local service was reachable at `http://127.0.0.1:3001/api/node/role`.
- `cargo-audit` is not installed locally, so `check-release-audit-gate.sh` used its diagnostic-only skip path; CI/release required mode remains fail-closed.
- No new unblocked Current Web/server `MUST` gap was found during this gate.

## Decision

The full regression gate is closed. The next batch should select the next small implementation or evidence target from the current plan without opening signing、store、physical-device readiness、native authority writes、Mobile process runtime、Android process runtime、Web Git writer or server-backed Settings API.

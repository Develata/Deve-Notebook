# Full Regression Gate Refresh After Web Shell Current Closure - 2026-05-17

本报告记录 Repo Switcher 与 PWA Manifest 当前 Web shell 合同闭合后的全量回归闸门。`docs/plan/` 未修改。

## Scope

- Closed Web shell batches: Repo Switcher, PWA Manifest.
- Purpose: verify workspace-wide compile, test, lint, runtime, release, native/mobile boundary, and domain guard health before selecting another feature batch.
- Boundaries kept closed: Web Git writer, server-backed Settings API, native process runtime, signing, physical-device release, native authority writes.

## Cargo Gates

Ran:

- `cargo fmt --check`
- `cargo test --locked`
- `cargo clippy --all-targets --all-features -- -D warnings`

Results:

- Format: passed.
- Full workspace tests: passed.
- All-feature clippy: passed.

## Guard Scripts

Ran:

- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-architecture-registry.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-foundation-baseline.sh`
- `bash scripts/check-network-baseline.sh`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/check-cli-settings-baseline.sh`
- `bash scripts/check-storage-repo-baseline.sh`
- `bash scripts/check-rendering-baseline.sh`
- `bash scripts/check-ai-baseline.sh`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/check-settings-local-feedback-baseline.sh`
- `bash scripts/check-repo-file-ops-baseline.sh`
- `bash scripts/check-search-baseline.sh`
- `bash scripts/check-graph-baseline.sh`
- `bash scripts/check-mobile-baseline.sh`
- `bash scripts/check-browser-prefs-boundary.sh`
- `bash scripts/check-large-doc-baseline.sh`
- `bash scripts/check-ui-spa-routing-baseline.sh`
- `bash scripts/check-ui-dashboard-refresh-baseline.sh`
- `bash scripts/check-ui-disconnect-baseline.sh`
- `bash scripts/check-ui-desktop-baseline.sh`
- `bash scripts/check-ui-focus-baseline.sh`
- `bash scripts/check-ui-token-baseline.sh`
- `bash scripts/check-ui-z-index-baseline.sh`
- `bash scripts/check-diff-color-baseline.sh`
- `bash scripts/check-i18n-formatting-baseline.sh`
- `bash scripts/check-i18n-hardcoded-baseline.sh`
- `bash scripts/check-ws-structured-errors.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-release-audit-gate.sh`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/check-mobile-platform-package-preflight.sh`
- `bash scripts/check-dev-data-health-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `bash scripts/check-source-control-smoke-hygiene.sh`
- `bash scripts/plan-coverage.sh`
- `bash scripts/smoke-web-release-build.sh`
- `bash scripts/smoke-runtime-happy-path.sh`
- `bash scripts/smoke-runtime-recovery-path.sh`
- `bash scripts/smoke-runtime-release-info.sh`
- `git diff --check`

Results:

- Acceptance bindings: automated `149`, feature walkthrough `54`, manual `0`, unbound soft `0`.
- Architecture registry: `72` flows, active drift `0`.
- Feature operation paths: passed.
- Plan coverage: blocking violations `0`, soft warnings `18`, dangling `plan_ref` `0`, i18n leaks `0`.
- Domain baselines: passed.
- Release audit: passed; local `cargo-audit` was unavailable and skipped in diagnostic mode, while `npm audit` reported `0` vulnerabilities.
- Web release build: passed.
- Native/mobile boundary guards: passed.
- Mobile package preflight: passed in non-required target-host mode; iOS target-host build remained skipped on Linux as expected.
- Runtime happy/recovery smoke: passed.
- Runtime release-info smoke: skipped because no local service was running at `http://127.0.0.1:3001/api/node/role`; this is an environment condition, not a product regression.
- Diff hygiene: passed.

## Decision

Full Regression Gate Refresh After Web Shell Current Closure is green.

Next batch: **Mainline Gap Rescan After Web Shell Current Closure**.

Rationale:

- Web shell current closures did not introduce compile, lint, runtime, release, native/mobile, plan coverage, or domain guard drift.
- The next step should re-scan `docs/plan/`, `docs/features/`, acceptance cases, and code before selecting another implementation slice.
- Platform post-gate work remains deferred unless the rescan explicitly selects it under existing Desktop/Android boundaries.

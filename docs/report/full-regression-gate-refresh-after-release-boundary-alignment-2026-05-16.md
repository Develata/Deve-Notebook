# Full Regression Gate Refresh After Release Boundary Alignment

Date: 2026-05-16

## Scope

- Source of truth: `docs/plan/`.
- Regression scope: full Rust test suite, all-feature lint, formatting, release/native/mobile/domain guards, architecture/acceptance/plan coverage, and runtime smoke.
- Non-goal: reopening native process runtime, native authority writes, signing/store release, or physical-device readiness.

## Result

- Application regression gate: pass.
- `docs/plan/` changes: none.
- Code changes during gate: none.
- Existing soft warnings: `plan-coverage.sh` still reports 17 file-size soft warnings and 0 blocking violations.
- Environment blocker: local Docker release smoke is blocked by host Docker/WSL instability. The first `scripts/smoke-docker-release.sh` run hit a Docker/BuildKit `SIGBUS` during image build; subsequent `docker info` / `docker ps` also triggered `SIGBUS`. A `DOCKER_BUILDKIT=0` retry skipped because the Docker daemon was not reachable. This is not evidence of an application regression.

## Verification

Rust and formatting:

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `git diff --check`

Coverage and registry:

- `bash scripts/check-architecture-registry.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/plan-coverage.sh`

Domain guards:

- `bash scripts/check-network-baseline.sh`
- `bash scripts/check-storage-repo-baseline.sh`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/check-rendering-baseline.sh`
- `bash scripts/check-mobile-baseline.sh`
- `bash scripts/check-graph-baseline.sh`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/check-ai-baseline.sh`
- `bash scripts/check-search-baseline.sh`
- `bash scripts/check-cli-settings-baseline.sh`
- `bash scripts/check-auth-unauthorized-state.sh`
- `bash scripts/check-browser-prefs-boundary.sh`
- `bash scripts/check-dev-data-health-baseline.sh`
- `bash scripts/check-diff-color-baseline.sh`
- `bash scripts/check-i18n-formatting-baseline.sh`
- `bash scripts/check-i18n-hardcoded-baseline.sh`
- `bash scripts/check-large-doc-baseline.sh`
- `bash scripts/check-source-control-smoke-hygiene.sh`
- `bash scripts/check-ui-dashboard-refresh-baseline.sh`
- `bash scripts/check-ui-desktop-baseline.sh`
- `bash scripts/check-ui-disconnect-baseline.sh`
- `bash scripts/check-ui-focus-baseline.sh`
- `bash scripts/check-ui-spa-routing-baseline.sh`
- `bash scripts/check-ui-token-baseline.sh`
- `bash scripts/check-ui-z-index-baseline.sh`
- `bash scripts/check-ws-structured-errors.sh`

Release and platform guards:

- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `bash scripts/check-release-audit-gate.sh`
- `bash scripts/smoke-web-release-build.sh`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/check-desktop-package-preflight.sh`
- `bash scripts/check-desktop-target-host-preflight.sh`
- `bash scripts/check-desktop-package-startup-smoke.sh`
- `bash scripts/check-desktop-installer-smoke.sh`
- `bash scripts/check-mobile-platform-package-preflight.sh`
- `bash scripts/check-mobile-android-shell-package-build.sh`
- `bash scripts/check-mobile-ios-shell-package-build.sh`
- `bash scripts/check-mobile-android-install-startup-smoke.sh`
- `bash scripts/check-mobile-ios-install-startup-smoke.sh`
- `bash scripts/check-mobile-android-emulator-install-startup-smoke.sh`

Runtime smoke:

- `bash scripts/smoke-runtime-happy-path.sh`
- `bash scripts/smoke-runtime-recovery-path.sh`
- `bash scripts/smoke-runtime-release-info.sh` skipped because no runtime server was listening on `127.0.0.1:3001`.

Docker release smoke:

- `bash scripts/smoke-docker-release.sh` blocked by Docker/BuildKit `SIGBUS`.
- `DOCKER_BUILDKIT=0 bash scripts/smoke-docker-release.sh` skipped because Docker daemon was not reachable.

## Decision

The mainline code regression gate is green. The next batch should close Docker release smoke on a healthy Docker daemon or CI runner, then return to a mainline gap scan.

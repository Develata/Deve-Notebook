# Full Regression Gate Refresh After Local Windows / Android / WSL Evidence - 2026-05-21

## Scope

- Host scope: local Windows target host plus WSL2 Ubuntu ext4 clone.
- Evidence prerequisite: `docs/report/local-windows-android-wsl-evidence-refresh-2026-05-21.md`.
- Explicitly closed: Apple/macOS/iOS evidence, signing, store release, physical-device readiness, native authority writes, Mobile process runtime, Android process runtime, Web Git writer, and server-backed Settings API.
- `docs/plan/` was not modified.

## Baseline

- Current regression baseline: `9d957f25 Record local Windows Android WSL evidence refresh`.
- Code evidence baseline from the target-host route: `c7534fce Fix Android shell package build environment`.
- WSL ext4 clone baseline: `9d957f257 Record local Windows Android WSL evidence refresh`.

## Host And Tool Versions

- Windows: `Windows 10 Home China 25H2`, build `26200.8457`.
- Windows Git Bash host reported by gates: `MSYS_NT-10.0-26200`.
- Windows Rust/Cargo: `rustc 1.94.0 (4a4ef493e 2026-03-02)`, `cargo 1.94.0`, host `x86_64-pc-windows-msvc`.
- Windows Node/Trunk/Tauri CLI: `node v24.15.0`, `trunk 0.21.14`, `tauri-cli 2.11.2`.
- WSL Rust/Cargo: `rustc 1.92.0`, `cargo 1.92.0`, host `x86_64-unknown-linux-gnu`.
- WSL Node/Trunk/Tauri CLI: `node v18.19.1`, `trunk 0.21.14`, `tauri-cli 2.11.1`.

## Commands Run

### Core Rust And Hygiene

```bash
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
git diff --check
git status --short
```

Result:

- `cargo fmt --check`: passed.
- `cargo test --locked`: passed.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: passed.
- `git diff --check`: passed.
- `git status --short`: clean before report writing.

### Architecture / Plan / Acceptance Gates

```bash
./scripts/check-acceptance-bindings.sh
./scripts/check-feature-operation-paths.sh
./scripts/check-architecture-registry.sh
./scripts/plan-coverage.sh
```

Result:

- Acceptance bindings: passed, with `149` automated bindings, `54` feature walkthrough bindings, `0` manual bindings, and `0` unbound acceptance cases.
- Feature operation paths: passed.
- Architecture registry: passed, `72 flows`, `0 active drift`.
- Plan coverage: passed with `0` blocking violations and `27` soft warnings.
- Note: `check-architecture-registry.sh` and `plan-coverage.sh` timed out when first run in a Windows Git Bash parallel batch. They were rerun serially in the WSL ext4 clone and passed; this was treated as an execution-environment timeout, not a repository gate failure.

### Domain / UI / Release Baselines

```bash
./scripts/check-release-baseline.sh
./scripts/check-dev-runbook-baseline.sh
./scripts/check-dev-data-health-baseline.sh
./scripts/check-foundation-baseline.sh
./scripts/check-network-baseline.sh
./scripts/check-auth-baseline.sh
./scripts/check-auth-unauthorized-state.sh
./scripts/check-ws-structured-errors.sh
./scripts/check-storage-repo-baseline.sh
./scripts/check-repo-file-ops-baseline.sh
./scripts/check-source-control-baseline.sh
./scripts/check-source-control-smoke-hygiene.sh
./scripts/check-settings-local-feedback-baseline.sh
./scripts/check-cli-settings-baseline.sh
./scripts/check-browser-prefs-boundary.sh
./scripts/check-search-baseline.sh
./scripts/check-graph-baseline.sh
./scripts/check-rendering-baseline.sh
./scripts/check-large-doc-baseline.sh
./scripts/check-diff-color-baseline.sh
./scripts/check-ui-dashboard-refresh-baseline.sh
./scripts/check-ui-desktop-baseline.sh
./scripts/check-ui-disconnect-baseline.sh
./scripts/check-ui-focus-baseline.sh
./scripts/check-ui-spa-routing-baseline.sh
./scripts/check-ui-token-baseline.sh
./scripts/check-ui-z-index-baseline.sh
./scripts/check-i18n-formatting-baseline.sh
./scripts/check-i18n-hardcoded-baseline.sh
./scripts/check-ai-baseline.sh
./scripts/check-mobile-baseline.sh
./scripts/check-native-track-boundary.sh
./scripts/check-release-audit-gate.sh
```

Result:

- All listed baseline scripts passed.
- `check-release-audit-gate.sh`: passed in local diagnostic mode; `cargo-audit` was unavailable, so the script reported the audit step as skipped while keeping the gate `ok`.

### Native / Mobile Gates

```bash
./scripts/check-native-process-adapter-gate.sh
./scripts/check-native-packaging-gate.sh

DEVE_MOBILE_PACKAGE_TARGETS=android \
DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1 \
DEVE_MOBILE_ANDROID_PACKAGE_TARGET=x86_64 \
./scripts/check-mobile-platform-package-preflight.sh
```

Result:

- Native process adapter gate: passed.
- Native packaging gate: passed.
- Android-scoped mobile platform package preflight on Windows Git Bash: passed.
- Key native-packaging note: WSL Linux desktop native-session package smoke still emits missing `libayatana-appindicator3` / `libappindicator3` warnings, then follows the script's package-sidecar skip path; overall `native-packaging-gate-check: ok`.

### Web / Runtime / Docker Smoke

```bash
./scripts/smoke-web-release-build.sh
./scripts/smoke-runtime-happy-path.sh
./scripts/smoke-runtime-recovery-path.sh
./scripts/smoke-runtime-release-info.sh
./scripts/smoke-docker-release.sh
```

Result:

- Web release build: passed with `trunk 0.21.14`.
- Runtime happy-path smoke: passed.
- Runtime recovery smoke: passed.
- Runtime release-info smoke: skipped because no server was listening at `http://127.0.0.1:3001/api/node/role`; this is the expected local diagnostic behavior with `DEVE_RUNTIME_SMOKE_REQUIRED=0`.
- Docker release smoke: passed; local image build, production-auth container startup, `/api/node/role`, and login probe all succeeded.

## Target-host Evidence Reused

The target-host evidence immediately preceding this full regression remains valid for this regression baseline because the only later commit before this run was a docs/report update:

- Windows Desktop required target-host preflight, NSIS package build, startup smoke, and installer smoke: passed.
- Android Studio x86_64 shell APK build: passed.
- Android 35 x86_64 emulator install/startup smoke: passed.
- WSL native process adapter and native packaging gates: passed and were rerun in this full regression.

See `docs/report/local-windows-android-wsl-evidence-refresh-2026-05-21.md` for artifact paths and Android emulator details.

## Failures And Skips

- Blocking failures: none.
- Diagnostic skips:
  - `smoke-runtime-release-info.sh`: skipped because no existing local server was reachable on port `3001`.
  - `check-release-audit-gate.sh`: `cargo-audit` binary unavailable, so local diagnostic audit was skipped by script policy.
- Non-repo execution notes:
  - The first Windows Git Bash parallel run of architecture/plan gates timed out; WSL serial reruns passed.
  - A malformed ad hoc Bash loop in PowerShell expanded `$s` before Bash received it; it was discarded and not counted as a gate result.

## Conclusion

Local full regression gate is closed on `9d957f25` for the current non-Apple route. Windows Desktop target-host, Android Studio/emulator target-host, WSL native gates, Web release, Docker release, runtime happy/recovery, architecture/plan coverage, and domain/UI baselines are all green under the explicit scope above.

Apple/macOS/iOS evidence remains paused until a real Apple-capable target host is available.

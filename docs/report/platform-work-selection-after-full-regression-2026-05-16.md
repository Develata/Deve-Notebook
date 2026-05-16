# Platform Work Selection After Full Regression - 2026-05-16

本报告记录 full regression gate 之后的平台工作选择。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/08_ui_design_03_mobile.md`, `docs/plan/15_release.md`.
- Evidence inputs: `docs/report/platform-distribution-readiness-triage-2026-05-16.md`, `docs/report/platform-artifact-consumption-runbook-2026-05-16.md`, `docs/report/docker-release-smoke-ci-follow-up-2026-05-16.md`.
- Current head: `6c78f2fb`.

## Constraints

- Docker release path remains `deve_cli` direct locked release build inside OCI image.
- Desktop and mobile remain shell/service binding layers; they must not become business authority.
- Native process runtime remains closed unless an explicit post-gate runtime feature opens it.
- Native authority writes remain closed.

## Evidence State

- Docker Smoke GitHub run `25963571993` passed on `f3f23e1e`.
- Native Target Host GitHub run `25960266472` passed on `9439c864`.
- Current `HEAD` is newer than both platform evidence runs.
- Existing evidence proves the platform shell paths, but not current-head readiness.

## Selection

The next executable platform batch is **Current HEAD Platform Evidence Refresh**.

The batch should trigger and collect:

- `.github/workflows/docker-smoke.yml` on current `HEAD`.
- `.github/workflows/native-target-host.yml` on current `HEAD` with:
  - `target=all`
  - `required_preflight=true`
  - `run_desktop_package_build=true`
  - `run_desktop_startup_smoke=true`
  - `run_desktop_installer_smoke=true`
  - `run_mobile_android_package_build=true`
  - `run_mobile_android_install_startup_smoke=true`
  - `run_mobile_ios_package_build=true`
  - `run_mobile_ios_install_startup_smoke=true`

Evidence collection must use `scripts/collect-native-target-host-evidence.sh`.

## Non-Goals

- No native process runtime implementation.
- No native authority write path.
- No macOS notarization, Windows signing, Android Play Store, iOS TestFlight/App Store, or physical-device readiness claim.
- No broad UI or platform abstraction rewrite.

## Exit Criteria

- Docker release smoke is green on current `HEAD`.
- Desktop macOS and Windows package/startup/installer smoke are green on current `HEAD`.
- Android emulator package/install/startup smoke is green on current `HEAD`.
- iOS simulator package/install/startup smoke is green on current `HEAD`.
- Evidence artifacts preserve `Process runtime gate: closed` and `Native authority writes: closed`.

If these pass, the next platform decision can choose between signing/notarization, Android physical-device or signed package gate, iOS signing/TestFlight gate, or returning to mainline implementation gaps.

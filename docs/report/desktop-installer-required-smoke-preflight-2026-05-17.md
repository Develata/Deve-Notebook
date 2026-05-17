# Desktop Installer Required Smoke Preflight - 2026-05-17

本报告记录 Desktop installer required smoke 的本地 preflight hardening。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/15_release.md`.
- Input queue: `docs/report/post-regression-work-selection-after-desktop-native-session-evidence-closure-2026-05-17.md`.
- Boundary: Desktop target-host installer smoke workflow、required-mode invalid request、evidence field validation。
- Non-goal: signing、store、physical-device readiness、native authority writes、Android process runtime、Web Git writer、server-backed Settings API。

## Changes

- Desktop macOS / Windows target-host workflow now rejects startup/native-session smoke requests when `run_desktop_package_build=false`.
- Desktop macOS / Windows target-host workflow now rejects installer smoke requests when `run_desktop_package_build=false`.
- Dispatch helper now rejects the same invalid Desktop startup/installer request before sending a GitHub workflow dispatch.
- Desktop target-host evidence now records `invalid_startup_request=` and `invalid_installer_request=`.
- Evidence validator now requires Desktop reports to include `desktop_preflight=`, `process_gate=`, invalid-request fields, `package_build=`, `startup_smoke=`, `native_session_smoke=` and `installer_smoke=`.
- Release baseline guard now checks the invalid installer request gate and Desktop installer evidence fields.

## Local Validation

- `bash -n scripts/check-native-target-host-evidence.sh scripts/check-desktop-installer-smoke.sh scripts/check-release-baseline.sh`
- `bash scripts/check-native-target-host-evidence.sh`
- `bash scripts/check-desktop-installer-smoke.sh`
- `DEVE_DESKTOP_INSTALLER_SMOKE_REQUIRED=1 bash scripts/check-desktop-installer-smoke.sh`: expected fail-closed on non-macOS/non-Windows host.
- Desktop evidence validator positive fixture: passed.
- Desktop evidence validator negative fixture without invalid-request fields: expected fail-closed.
- Dispatch helper invalid Desktop installer request fixture: expected fail-closed.
- `bash scripts/check-release-baseline.sh`
- `.github/workflows/native-target-host.yml` YAML parse: passed.
- `git diff --check`

## Decision

The local preflight is closed. The next batch should dispatch Desktop macOS and Windows target-host installer smoke with package build enabled, then collect and record evidence.

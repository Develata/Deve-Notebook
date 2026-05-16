# Platform Distribution Readiness Triage - 2026-05-16

本报告记录 Docker、Desktop、Android、iOS 平台发布面当前证据。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/08_ui_design_03_mobile.md`, `docs/plan/14_tech_stack.md`, `docs/plan/15_release.md`.
- Code scope: `.github/workflows/native-target-host.yml`, platform gate scripts, Docker release smoke.
- Non-goal: 打开 native process runtime、native authority writes、physical-device release readiness、store distribution 或 signing/notarization release gate。

## Local Verification

Ran:

- `scripts/check-release-baseline.sh`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-packaging-gate.sh`
- `scripts/check-mobile-platform-package-preflight.sh`
- `scripts/check-desktop-package-preflight.sh`
- `scripts/check-desktop-package-startup-smoke.sh`
- `scripts/check-mobile-android-shell-package-build.sh`
- `scripts/check-mobile-ios-shell-package-build.sh`
- `bash -n scripts/check-desktop-target-host-preflight.sh scripts/check-release-baseline.sh`
- `shellcheck scripts/check-desktop-target-host-preflight.sh scripts/check-release-baseline.sh`
- `cargo fmt --check`
- `git diff --check`
- `DEVE_DOCKER_SMOKE_REQUIRED=1 scripts/smoke-docker-release.sh`

Results:

- Local release/native/mobile/Desktop gates: pass.
- Docker image build and production container smoke: pass.
- Docker `/api/node/role` and production login smoke: pass.

## Target-host Evidence

Initial all-platform run `25959925600` exposed two target-host preflight issues:

- macOS unsigned package smoke was blocked by missing signing secrets.
- Windows required `cl.exe` in Bash `PATH` and did not install/announce NSIS before preflight.

Fix commit: `9439c864`.

Follow-up all-platform run:

- Run: `25960266472`
- URL: https://github.com/Develata/Deve-Notebook/actions/runs/25960266472
- Head: `9439c86463df29530dd04a265855861914ab7b12`
- Result: success.

Evidence artifacts:

| Target | Result |
| --- | --- |
| Desktop macOS | `desktop_preflight=success`, `package_build=success`, `startup_smoke=success`, `installer_smoke=success` |
| Desktop Windows | `desktop_preflight=success`, `package_build=success`, `startup_smoke=success`, `installer_smoke=success` |
| Mobile Android | `mobile_android_preflight=success`, `package_build=success`, `install_startup_smoke=success` |
| Mobile iOS | `mobile_ios_preflight=success`, `package_build=success`, `install_startup_smoke=success` |

All evidence artifacts preserve:

- `Process runtime gate: closed`
- `Native authority writes: closed`

## Fixes

- macOS preflight now honors `DEVE_DESKTOP_PACKAGE_NO_SIGN=1`; unsigned shell package smoke no longer requires release signing secrets.
- Windows workflow installs NSIS before Desktop package preflight when package build is requested.
- Windows preflight accepts Visual Studio Build Tools detection via `vswhere` instead of requiring `cl.exe` to be directly visible in Bash `PATH`.
- Release baseline now guards the updated Desktop preflight prerequisites.

## Decision

Current shell-only platform distribution evidence is closed for Docker, Desktop macOS/Windows, Android emulator, and iOS simulator.

Remaining platform work is not implementation-blocking:

- signed macOS release, notarization/Gatekeeper release policy;
- Windows signed installer policy;
- Android Play Store / signed release packaging;
- iOS device signing and App Store/TestFlight policy;
- physical-device install/startup evidence.

Those gates must remain closed until explicitly opened.

## Next

Next executable batch should produce a platform artifact consumption/runbook pass: how to obtain and smoke-test Docker, Desktop, Android, and iOS artifacts, with explicit wording that current evidence is shell-only and does not claim store/signing/physical-device readiness.

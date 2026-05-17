# Platform Evidence Refresh After Web Shell Current Closure - 2026-05-17

本报告记录 Web shell current closure 后的 Docker 与 shell-only native target-host evidence。`docs/plan/` 未修改。

## Scope

- Selection report: `docs/report/mainline-gap-rescan-after-web-shell-current-closure-2026-05-17.md`.
- Runtime dispatch head: `818b5c9406c4533c1a74bdbbfce6e41db7146fd7`.
- Android evidence clarification head: `546fd7e2f5f4b0dcdf1c2b56ddeb0ea673badda9`.
- Non-goal: 打开 native process runtime、native authority writes、signing/notarization、store release 或 physical-device readiness。

`546fd7e2` 只修改 Native Target Host workflow 的 Android evidence 标记，不修改 runtime/app code。Docker、Desktop macOS、Desktop Windows 与 Mobile iOS 的 runtime evidence 仍以 `818b5c94` 为准；Android evidence 在最终 workflow head `546fd7e2` 重新验证。

## GitHub Runs

| Surface | Run | Head | Result |
| --- | --- | --- | --- |
| Docker Smoke | `25980828115` | `818b5c94` | success |
| Native Target Host all | `25980828117` | `818b5c94` | success |
| Native Target Host Android rerun | `25981241624` | `546fd7e2` | success |

URLs:

- https://github.com/Develata/Deve-Notebook/actions/runs/25980828115
- https://github.com/Develata/Deve-Notebook/actions/runs/25980828117
- https://github.com/Develata/Deve-Notebook/actions/runs/25981241624

## Docker Result

Run `25980828115` completed successfully.

- `docker-release-smoke`: success.
- `Run Docker release smoke`: success.

This refreshes the Docker release image build and production smoke evidence for the current runtime head.

## Native Target-host Result

Run `25980828117` completed successfully.

| Target | Result | Evidence |
| --- | --- | --- |
| Desktop macOS | success | package build, startup smoke, installer install/uninstall smoke |
| Desktop Windows | success | package build, startup smoke, installer install/uninstall smoke |
| Mobile Android | success | emulator install/startup smoke |
| Mobile iOS | success | shell package build, simulator install/startup smoke |

Downloaded and validated full-run evidence artifacts with:

```bash
DEVE_NATIVE_TARGET_HOST_RUN_ID=25980828117 \
  DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 \
  DEVE_NATIVE_TARGET_HOST_EVIDENCE_DIR=target/native-target-host-evidence-download-25980828117 \
  scripts/collect-native-target-host-evidence.sh
```

Validator result:

- `desktop-macos.md`: ok.
- `desktop-windows.md`: ok.
- `mobile-android.md`: ok.
- `mobile-ios.md`: ok.

## Android Evidence Clarification

During the all-target run, the Android standalone package-build step was skipped because emulator install/startup smoke owns the `x86_64` debug package build path. That behavior was correct but easy to misread in the workflow UI.

Commit `546fd7e2` adds a small evidence-only step:

- `Mobile Android shell package build covered by emulator smoke`

Android rerun `25981241624` completed successfully.

- `Mobile Android emulator install/startup smoke`: success.
- `Mobile Android shell package build covered by emulator smoke`: success.
- `Upload Mobile Android package artifacts`: success.
- `Write Mobile Android target-host evidence`: success.

Downloaded and validated corrected Android evidence with:

```bash
DEVE_NATIVE_TARGET_HOST_RUN_ID=25981241624 \
  DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 \
  DEVE_NATIVE_TARGET_HOST_EVIDENCE_ARTIFACTS=deve-native-target-host-evidence-android \
  DEVE_NATIVE_TARGET_HOST_EVIDENCE_DIR=target/native-target-host-evidence-download-25981241624 \
  scripts/collect-native-target-host-evidence.sh
```

Corrected Android evidence includes:

- `package_build_command=covered by emulator install/startup smoke: DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1 scripts/check-mobile-android-shell-package-build.sh`
- `mobile_android_preflight=success`
- `process_gate=success`
- `package_build=success`
- `install_startup_smoke=success`
- `Process runtime gate: closed`
- `Native authority writes: closed`

## Evidence Details

Desktop macOS evidence:

- `desktop_preflight=success`
- `process_gate=success`
- `package_build=success`
- `startup_smoke=success`
- `installer_smoke=success`
- `Process runtime gate: closed`
- `Native authority writes: closed`

Desktop Windows evidence:

- `desktop_preflight=success`
- `process_gate=success`
- `package_build=success`
- `startup_smoke=success`
- `installer_smoke=success`
- `Process runtime gate: closed`
- `Native authority writes: closed`

Mobile Android evidence:

- `mobile_android_preflight=success`
- `process_gate=success`
- `package_build=success`
- `install_startup_smoke=success`
- `Process runtime gate: closed`
- `Native authority writes: closed`

Mobile iOS evidence:

- `mobile_ios_preflight=success`
- `process_gate=success`
- `package_build=success`
- `install_startup_smoke=success`
- `Process runtime gate: closed`
- `Native authority writes: closed`

## Result

The current runtime head has refreshed shell-only evidence for:

- Docker release smoke.
- Desktop macOS package/startup/installer smoke.
- Desktop Windows package/startup/installer smoke.
- Android emulator install/startup smoke.
- iOS simulator install/startup smoke.

This does not claim signed release, app store, notarization, TestFlight, Play Store, physical-device readiness, native process runtime, or native authority writes.

## Next

Return to mainline feature implementation selection. Do not open post-gate platform runtime unless a later selection report explicitly chooses it.

# Current HEAD Platform Evidence Refresh - 2026-05-16

本报告记录当前 `HEAD` 的 Docker、Desktop、Android、iOS shell-only platform evidence。`docs/plan/` 未修改。

## Scope

- Selection report: `docs/report/platform-work-selection-after-full-regression-2026-05-16.md`.
- Dispatch head: `154fcc9140c08016975e7778fdaadf9f647e7298`.
- Non-goal: 打开 native process runtime、native authority writes、signing/notarization、store release 或 physical-device readiness。

## GitHub Runs

| Surface | Run | Result |
| --- | --- | --- |
| Docker Smoke | `25966339253` | success |
| Native Target Host | `25966339263` | success |

URLs:

- https://github.com/Develata/Deve-Notebook/actions/runs/25966339253
- https://github.com/Develata/Deve-Notebook/actions/runs/25966339263

## Docker Result

Run `25966339253` completed on current `HEAD`.

Job result:

- `docker-release-smoke`: success.
- `Run Docker release smoke`: success.

This refreshes the Docker release image build and production smoke evidence on current `HEAD`.

## Native Target-host Result

Run `25966339263` completed on current `HEAD`.

Job results:

| Target | Result | Evidence |
| --- | --- | --- |
| Desktop macOS | success | package build, startup smoke, installer install/uninstall smoke |
| Desktop Windows | success | package build, startup smoke, installer install/uninstall smoke |
| Mobile Android | success | emulator install/startup smoke |
| Mobile iOS | success | shell package build, simulator install/startup smoke |

Downloaded and validated evidence artifacts with:

```bash
DEVE_NATIVE_TARGET_HOST_RUN_ID=25966339263 \
  DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 \
  scripts/collect-native-target-host-evidence.sh
```

Validator result:

- `desktop-macos.md`: ok.
- `desktop-windows.md`: ok.
- `mobile-android.md`: ok.
- `mobile-ios.md`: ok.

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

The Android workflow skips the standalone package-build step when emulator install/startup smoke is requested, because that smoke owns the `x86_64` debug package build path. The evidence therefore records `package_build=success` from the install/startup smoke outcome.

Mobile iOS evidence:

- `mobile_ios_preflight=success`
- `process_gate=success`
- `package_build=success`
- `install_startup_smoke=success`
- `Process runtime gate: closed`
- `Native authority writes: closed`

## Result

Current `HEAD` has refreshed shell-only evidence for:

- Docker release smoke.
- Desktop macOS package/startup/installer smoke.
- Desktop Windows package/startup/installer smoke.
- Android emulator install/startup smoke.
- iOS simulator install/startup smoke.

This does not claim signed release, app store, notarization, TestFlight, Play Store, or physical-device readiness.

## Next

Return to mainline gap scan before opening any post-gate platform runtime. If the next platform batch is selected explicitly, the candidates are:

- Desktop signing/notarization policy and smoke.
- Windows signed installer policy and smoke.
- Android signed release or physical-device smoke.
- iOS signing/TestFlight/device smoke.

# Android Target-host Evidence Refresh After Desktop Installer Closure - 2026-05-18

本报告记录 Desktop installer evidence closure 后的 Android shell-only target-host evidence refresh。`docs/plan/` 未修改。

## Scope

- Selection report: `docs/report/post-regression-work-selection-after-desktop-installer-evidence-closure-2026-05-18.md`.
- Runtime head: `699e5bbd66b0eb751144bca3c7d55f881b75a912`.
- Target: `mobile-android`.
- Non-goal: signing、store、physical-device readiness、Android process runtime、native authority writes、Web Git writer、server-backed Settings API。

## Dispatch

```bash
DEVE_NATIVE_TARGET_HOST_DISPATCH=1 \
DEVE_NATIVE_TARGET_HOST_TARGET=mobile-android \
DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT=true \
DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_PACKAGE_BUILD=true \
DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE=true \
scripts/dispatch-native-target-host-workflow.sh
```

## Runs

| Run | Head | Result | Notes |
| --- | --- | --- | --- |
| `26023064624` | `0761d2d8` | failure | Android job was incorrectly blocked by Desktop Linux native-packaging test dependencies inside the generic process gate. |
| `26023546240` | `699e5bbd` | success | Android scoped process gate passed; emulator package/install/startup evidence passed. |

URLs:

- https://github.com/Develata/Deve-Notebook/actions/runs/26023064624
- https://github.com/Develata/Deve-Notebook/actions/runs/26023546240

## Fix

Commit `699e5bbd` scoped the mobile-android workflow process gate with:

- `DEVE_NATIVE_PROCESS_ADAPTER_RUN_DESKTOP_NATIVE_PACKAGING_TESTS=0`
- default `scripts/check-native-process-adapter-gate.sh` behavior still runs Desktop native-packaging tests.
- release baseline now guards both the scoped workflow field and the script option.

This avoids requiring Desktop Linux Tauri system libraries for Android target-host evidence while preserving the full local/default process gate.

## Evidence

Downloaded and validated:

```bash
DEVE_NATIVE_TARGET_HOST_RUN_ID=26023546240 \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_ARTIFACTS=deve-native-target-host-evidence-android \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_DIR=target/native-target-host-evidence-download-26023546240 \
scripts/collect-native-target-host-evidence.sh

bash scripts/check-native-target-host-evidence.sh \
  target/native-target-host-evidence-download-26023546240/deve-native-target-host-evidence-android/native-target-host-evidence/mobile-android.md
```

Result:

- `mobile_android_preflight=success`
- `process_gate=success`
- `invalid_request=skipped`
- `package_build=success`
- `install_startup_smoke=success`
- `Install result: success`
- `Startup result: success`
- `Process runtime gate: closed`
- `Native authority writes: closed`

## Local Verification

- `bash -n scripts/check-native-process-adapter-gate.sh scripts/check-release-baseline.sh scripts/dispatch-native-target-host-workflow.sh`
- `.github/workflows/native-target-host.yml` parsed as YAML.
- `bash scripts/check-release-baseline.sh`
- `DEVE_NATIVE_PROCESS_ADAPTER_RUN_DESKTOP_NATIVE_PACKAGING_TESTS=0 bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/plan-coverage.sh`: blocking violations `0`, dangling `plan_ref` `0`, i18n leaks `0`, soft warnings `27`.
- `git diff --check`

## Result

Android shell-only package build and emulator install/startup evidence is current at `699e5bbd`.

This does not claim signed release, Play Store readiness, physical-device readiness, Android process runtime, or native authority writes.

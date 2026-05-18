# Mobile iOS Target-host Evidence Refresh After Android Closure - 2026-05-18

本报告记录 Android shell-only evidence closure 后的 Mobile iOS target-host evidence refresh。`docs/plan/` 未修改。

## Scope

- Selection report: `docs/report/post-regression-work-selection-after-android-evidence-closure-2026-05-18.md`.
- Runtime head: `33ae0fa8d6241dc8646275d50ba9b089b15f8032`.
- Target: `mobile-ios`.
- Non-goal: signing、store、physical-device readiness、Mobile process runtime、native authority writes、Android process runtime、Web Git writer、server-backed Settings API。

## Dispatch

```bash
DEVE_NATIVE_TARGET_HOST_TARGET=mobile-ios \
DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT=true \
DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_IOS_PACKAGE_BUILD=true \
DEVE_NATIVE_TARGET_HOST_RUN_MOBILE_IOS_INSTALL_STARTUP_SMOKE=true \
DEVE_NATIVE_TARGET_HOST_DISPATCH=1 \
scripts/dispatch-native-target-host-workflow.sh
```

## Run

| Run | Head | Result | Notes |
| --- | --- | --- | --- |
| `26026170256` | `33ae0fa8` | success | iOS shell package build, simulator install/startup smoke, process gate, and evidence upload passed. |

URL:

- https://github.com/Develata/Deve-Notebook/actions/runs/26026170256

## Evidence

Downloaded and validated:

```bash
DEVE_NATIVE_TARGET_HOST_RUN_ID=26026170256 \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_ARTIFACTS=deve-native-target-host-evidence-ios \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_DIR=target/native-target-host-evidence-download-26026170256 \
DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 \
scripts/collect-native-target-host-evidence.sh

bash scripts/check-native-target-host-evidence.sh \
  target/native-target-host-evidence-download-26026170256/deve-native-target-host-evidence-ios/mobile-ios.md
```

Result:

- `mobile_ios_preflight=success`
- `process_gate=success`
- `package_build=success`
- `install_startup_smoke=success`
- `Install result: success`
- `Startup result: success`
- `Process runtime gate: closed`
- `Native authority writes: closed`

Package artifact:

- Artifact: `deve-mobile-ios-packages`.
- Local copy: `target/native-target-host-evidence-download-26026170256/deve-mobile-ios-packages`.
- App bundle: `target/native-target-host-evidence-download-26026170256/deve-mobile-ios-packages/build/arm64-sim/Deve Notebook.app`.
- App bundle size: `31M`.
- Package artifact size on disk: `260M`.

## Local Verification

- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-mobile-baseline.sh`
- `bash scripts/check-native-target-host-evidence.sh target/native-target-host-evidence-download-26026170256/deve-native-target-host-evidence-ios/mobile-ios.md`
- `git diff --check`

## Result

Mobile iOS shell-only package build and simulator install/startup evidence is current at `33ae0fa8`.

This does not claim signing, App Store readiness, physical-device readiness, Mobile process runtime, native authority writes, Web Git writer, or server-backed Settings API.

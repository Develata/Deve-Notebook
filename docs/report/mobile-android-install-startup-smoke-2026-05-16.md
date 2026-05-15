# Mobile Android Emulator Install/Startup Smoke - 2026-05-16

## Scope

Record Mobile Android emulator install/startup target-host evidence for the shell-only Mobile app.

This report does not declare Android store/release readiness or open native process runtime.

## Run

- GitHub run: `25934596796`
- Workflow: `native-target-host.yml`
- Target: `mobile-android`
- Commit: `b29d3770b9f0ccd11ed683d728d9f42ad22f5bfa`
- Evidence artifact: `deve-native-target-host-evidence-android`
- Package artifact: `deve-mobile-android-packages`
- Local evidence copy: `target/native-target-host-evidence-download/deve-native-target-host-evidence-android/native-target-host-evidence/mobile-android.md`
- Local emulator log: `target/native-target-host-evidence-download/deve-native-target-host-evidence-android/mobile-android-emulator-smoke/emulator.log`

## Evidence

- Host: Ubuntu Linux x86_64 GitHub runner.
- Rust: `1.92.0`.
- Tauri CLI: `2.11.1`.
- Node: `24.15.0`.
- npm: `11.12.1`.
- Android target: `x86_64`.
- Emulator AVD: `deve-mobile-smoke-api35-default-x86_64`.

Command results:

- `mobile_android_preflight=success`
- `process_gate=success`
- `package_build=success`
- `install_startup_smoke=success`
- `invalid_request=skipped`

## Result

Mobile Android emulator shell package build, install, and startup smoke are closed for the current shell-only boundary.

The evidence preserves:

- process runtime gate: closed;
- native authority writes: closed;
- no backend supervision ownership;
- no ledger/vault/source-control/search/Git/`.notegit` native write path.

## Fixes From Failed Runs

- `25928994697`: Android preflight failed before `adb`; fixed by installing SDK platform-tools before target-host preflight.
- `25929413803`: Linux host native-packaging check was not relevant to Android target-host evidence; fixed by skipping host Wry/GTK checks for Android.
- `25930409738`: emulator could not find the generated AVD; fixed by using an isolated `ANDROID_AVD_HOME` and verifying `emulator -list-avds` after creation.

## Remaining

Android signed release packaging, Play Store distribution decisions, and physical device install/startup remain separate future gates.

# Mobile Install/Startup Evidence Plan - 2026-05-15

## Scope

Define the next executable Mobile target-host gate after Android APK package execution and iOS simulator package build.

This batch does not declare Mobile release readiness.

## Current Evidence

- Android shell APK package execution is closed by the existing package build gate.
- iOS simulator shell package build is closed by GitHub run `25917428903`.
- Desktop macOS/Windows package startup and installer install/uninstall smoke are closed by target-host evidence.
- Native child-process runtime remains closed by `KeepClosedUntilExplicitRuntimeFeature`.

## Gap

Package build evidence does not prove device or simulator install/startup readiness.

Android install/startup smoke requires an installable signed or debug APK and a reachable emulator or device through `adb`.

iOS install/startup smoke requires a macOS host, built simulator `.app`, and a booted iOS simulator through `xcrun simctl`.

## Gate Shape

- `scripts/check-mobile-android-install-startup-smoke.sh` is the Android install/startup gate.
- `scripts/check-mobile-ios-install-startup-smoke.sh` is the iOS install/startup gate.
- Both scripts are diagnostic-only by default.
- Required mode must fail closed when target-host tools, artifacts, or devices are missing.
- Required mode must install and launch only the Mobile WebView shell.
- Required mode must not open child-process runtime, backend supervision, ledger/vault/source-control/search/Git, or `.notegit` authority writes.

## Required Mode

Android:

```bash
DEVE_MOBILE_ANDROID_INSTALL_STARTUP_SMOKE_REQUIRED=1 scripts/check-mobile-android-install-startup-smoke.sh
```

Use `DEVE_MOBILE_ANDROID_PACKAGE_DEBUG=1` when building an emulator-smoke APK. Use `DEVE_MOBILE_ANDROID_APK_PATH=/path/to/signed.apk` when using another signed APK.

iOS:

```bash
DEVE_MOBILE_IOS_INSTALL_STARTUP_SMOKE_REQUIRED=1 scripts/check-mobile-ios-install-startup-smoke.sh
```

## Result

Mobile install/startup has executable fail-closed gates, but it remains open until target-host evidence is collected from an Android emulator/device and a booted iOS simulator.

# Local Windows / Android / WSL Evidence Refresh - 2026-05-21

## Scope

- Host scope: local Windows target host, Android Studio / SDK target host, and WSL2 Ubuntu ext4 clone.
- Explicitly closed: Apple/macOS/iOS evidence, signing, store release, physical-device readiness, native authority writes, Mobile process runtime, Android process runtime, Web Git writer, and server-backed Settings API.
- Plan files were not modified.

## Baseline

- Evidence code baseline: `c7534fce Fix Android shell package build environment`
- Supporting commits in this route:
  - `a1286b44 Relax native packaging manifest gate format`
  - `4df28552 Fix native packaging schema dependency gate`
  - `d365ea5c Split desktop native process runtime modules`
  - `52d36e6e Harden plan registry coverage checks`
  - `1df53239 Move runtime skeleton registry to docs registry`

## Host And Tool Versions

- Windows: `Microsoft Windows NT 10.0.26200.0`
- Windows Git Bash host reported by gates: `MSYS_NT-10.0-26200`
- Windows Rust: `rustc 1.94.0 (4a4ef493e 2026-03-02)`, host `x86_64-pc-windows-msvc`
- WSL Rust/Cargo: `rustc 1.92.0`, `cargo 1.92.0`, host `x86_64-unknown-linux-gnu`
- Node: `v24.15.0`
- Trunk: `0.21.14`
- Tauri CLI: `tauri-cli 2.11.2`
- Android Studio JBR: OpenJDK `21.0.10`, selected from `/c/Users/QQ/scoop/apps/android-studio/current/jbr`
- Android SDK tools: `adb 1.0.41` / `37.0.0-14910828`, emulator `36.5.11.0`, sdkmanager `20.0`
- Android NDK used by Tauri: `28.2.13676358`

## Fixes Applied

- Desktop/Mobile `native-packaging` now explicitly enables `indexmap 1.9.3/std` for the `schemars 0.8 -> indexmap 1.x` path used by Tauri schema generation. This fixed the WSL/Linux `schemars` compile failure without opening new runtime authority.
- Native track boundary gate now checks `native-packaging` feature members by item rather than requiring the old one-line manifest format.
- Android shell package build now prepares Android Studio JBR in the parent build script and disables Kotlin incremental compilation on Windows by default. This fixed:
  - Gradle falling back to Java 8 JRE without `JAVA_COMPILER`.
  - Kotlin incremental cross-drive failure between Cargo registry on `C:` and generated Android project on `E:`.

## Commands Run

### WSL / Linux

```bash
cd ~/gitclone/Deve-Notebook
./scripts/check-native-process-adapter-gate.sh
./scripts/check-native-packaging-gate.sh
```

Result:

- `check-native-process-adapter-gate.sh`: passed.
- `check-native-packaging-gate.sh`: passed.
- Key note: Linux desktop native-session package smoke emitted missing `libayatana-appindicator3` / `libappindicator3` runtime warnings and then followed the script's package-sidecar skip path; the overall native packaging gate returned `native-packaging-gate-check: ok`.

### Windows Desktop

```bash
DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1 \
DEVE_DESKTOP_TARGET_HOSTS=windows \
DEVE_DESKTOP_PACKAGE_NO_SIGN=1 \
./scripts/check-desktop-target-host-preflight.sh

DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 \
DEVE_DESKTOP_PACKAGE_NO_SIGN=1 \
DEVE_DESKTOP_PACKAGE_BUNDLES=nsis \
./scripts/check-desktop-platform-package-build.sh

DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED=1 \
DEVE_DESKTOP_PACKAGE_BUNDLES=nsis \
./scripts/check-desktop-package-startup-smoke.sh

DEVE_DESKTOP_INSTALLER_SMOKE_REQUIRED=1 \
DEVE_DESKTOP_PACKAGE_BUNDLES=nsis \
./scripts/check-desktop-installer-smoke.sh
```

Result:

- Desktop package preflight: passed.
- Windows target-host preflight: passed.
- NSIS package build: passed.
- Startup smoke: passed.
- Installer smoke: passed.
- Package artifact: `target/release/bundle/nsis/Deve Notebook_0.0.1_x64-setup.exe`
- Installer smoke used isolated registry key `HKCU\Software\deve\Deve Notebook`, installed to `target/desktop-installer-smoke/.../DeveNotebookInstallerSmoke`, launched successfully, then uninstalled.

### Android Studio / Emulator

```bash
DEVE_MOBILE_PACKAGE_TARGETS=android \
DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1 \
DEVE_MOBILE_ANDROID_PACKAGE_TARGET=x86_64 \
./scripts/check-mobile-platform-package-preflight.sh

DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1 \
DEVE_MOBILE_ANDROID_PACKAGE_TARGET=x86_64 \
DEVE_MOBILE_ANDROID_PACKAGE_DEBUG=1 \
DEVE_MOBILE_ANDROID_PACKAGE_APK=1 \
DEVE_MOBILE_ANDROID_PACKAGE_AAB=0 \
./scripts/check-mobile-android-shell-package-build.sh

DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED=1 \
DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL=35 \
DEVE_MOBILE_ANDROID_EMULATOR_SYSTEM_TARGET=default \
DEVE_MOBILE_ANDROID_EMULATOR_ARCH=x86_64 \
DEVE_MOBILE_ANDROID_PACKAGE_TARGET=x86_64 \
./scripts/check-mobile-android-emulator-install-startup-smoke.sh
```

Result:

- Android target-host preflight: passed.
- Android shell debug APK build: passed.
- Android emulator install/startup smoke: passed.
- APK artifact: `apps/mobile/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`
- Emulator smoke installed Android 35 x86_64 system image as needed, launched `emulator-5554`, installed the APK, started `dev.deve.notebook.mobile` via `monkey`, and observed app pid `2346`.

## Skipped / Closed

- Apple/macOS/iOS target-host evidence: skipped in this local route.
- Android signing / store / physical-device readiness: skipped.
- Android process runtime and Mobile process runtime: still closed.
- Native authority writes: still closed.
- Web Git writer and server-backed Settings API: still closed.

## Follow-Up

- Next local step is current-head full regression refresh on top of `c7534fce` or newer.
- Apple evidence remains a separate real macOS target-host task; ordinary Linux cloud output is not sufficient for Apple closure.

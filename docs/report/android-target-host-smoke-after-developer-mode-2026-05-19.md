# Android Target-host Smoke After Developer Mode - 2026-05-19

本报告记录 Windows Developer Mode 启用后，Android shell package build 与 API 36 emulator install/startup smoke 的本地 target-host evidence。`docs/plan/` 未修改。

## Scope

- Runtime head: `ca00bd38 Record Android target host toolchain probe`.
- Host: Microsoft Windows `10.0.26200.8457`.
- Target: `mobile-android` shell-only package/build/install/startup evidence.
- Non-goal: signing、store、physical-device readiness、Android process runtime、native authority writes、Web Git writer、server-backed Settings API。

## Environment

- Android Studio: scoop `android-studio 2025.3.4.7`, build `AI-253.32098.37.2534.15336583`.
- Java/JBR: OpenJDK `21.0.10`, `javac 21.0.10`.
- Android command-line tools: `sdkmanager 20.0`.
- Android SDK:
  - `platform-tools 37.0.0`
  - `emulator 36.5.11`
  - `platforms;android-36`
  - `build-tools;36.0.0`
  - `system-images;android-36;default;x86_64`
  - `ndk;28.2.13676358`
- Rust: `rustc 1.94.0`, `cargo 1.94.0`.
- Rust targets: `aarch64-linux-android`, `x86_64-linux-android`, `aarch64-pc-windows-msvc`, `x86_64-pc-windows-msvc`, `wasm32-unknown-unknown`.
- Node: `v24.15.0`.
- Trunk: `0.21.14`.
- Tauri CLI: `tauri-cli 2.11.1`.
- AVD home for successful run: `C:\Users\QQ\.deve-mobile-android-avd`.

## Preconditions

Developer Mode was enabled on Windows and verified with a non-admin symlink probe:

```powershell
New-Item -ItemType SymbolicLink -Path target\android-symlink-probe\link.txt -Target target\android-symlink-probe\target.txt
```

Result: `LinkType=SymbolicLink`.

## Commands

Android release APK package build:

```bash
DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1 \
DEVE_MOBILE_ANDROID_PACKAGE_TARGET=aarch64 \
DEVE_MOBILE_ANDROID_PACKAGE_APK=1 \
DEVE_MOBILE_ANDROID_PACKAGE_AAB=0 \
./scripts/check-mobile-android-shell-package-build.sh
```

API 36 emulator install/startup smoke:

```bash
DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED=1 \
DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL=36 \
DEVE_MOBILE_ANDROID_EMULATOR_SYSTEM_TARGET=default \
DEVE_MOBILE_ANDROID_EMULATOR_ARCH=x86_64 \
DEVE_MOBILE_ANDROID_AVD_HOME=/c/Users/QQ/.deve-mobile-android-avd \
DEVE_MOBILE_ANDROID_PACKAGE_TARGET=x86_64 \
./scripts/check-mobile-android-emulator-install-startup-smoke.sh
```

Both commands used temporary shell environment:

```bash
ANDROID_HOME=/c/Users/QQ/scoop/apps/android-clt/current
ANDROID_SDK_ROOT=$ANDROID_HOME
ANDROID_NDK_HOME=$ANDROID_HOME/ndk/28.2.13676358
NDK_HOME=$ANDROID_NDK_HOME
JAVA_HOME=/c/Users/QQ/scoop/apps/android-studio/current/jbr
PATH=/e/gitclone/Deve-Notebook/target/native-tools/bin:$JAVA_HOME/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH
```

Because Git Bash does not resolve `sdkmanager.bat` / `avdmanager.bat` as bare commands, the emulator smoke command exported temporary Bash wrappers for those two Android tools. No repository script was modified.

## Passed

- Symlink probe: passed after Developer Mode was enabled.
- `scripts/check-mobile-android-shell-package-build.sh`: passed for `aarch64` release APK.
- `scripts/check-mobile-android-emulator-install-startup-smoke.sh`: passed with API 36 x86_64 emulator.
- Nested gates inside emulator smoke passed:
  - `scripts/check-native-track-boundary.sh`
  - `scripts/check-mobile-platform-package-preflight.sh`
  - `cargo check --locked -p deve_mobile --no-default-features`
  - `cargo check --locked -p deve_mobile --features native-packaging`
  - `cargo test --locked -p deve_mobile --features native-packaging packaging -- --nocapture`
  - `scripts/check-mobile-android-shell-package-build.sh`
  - `scripts/check-mobile-android-install-startup-smoke.sh`
- Mobile packaging tests: `7 passed`.
- Install/startup smoke:
  - `Performing Streamed Install`
  - `Success`
  - `mobile-android-install-startup-smoke-check: app_id=dev.deve.notebook.mobile pid=2485`
  - `mobile-android-install-startup-smoke-check: serial=emulator-5554`
  - `mobile-android-install-startup-smoke-check: ok`
  - `mobile-android-emulator-install-startup-smoke-check: ok`

## Artifacts

- Release APK: `apps/mobile/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk`
  - Size: `56,440,509` bytes.
- Debug APK used by emulator smoke: `apps/mobile/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`
  - Size: `154,855,890` bytes.
- Emulator log: `target/mobile-android-emulator-smoke/emulator.log`.
- AVD: `deve-mobile-smoke-api36-default-x86_64`.

## Key Logs

Developer Mode unblocked Tauri's native-library symlink step:

```text
Info symlinking lib "E:\\gitclone\\Deve-Notebook\\target\\x86_64-linux-android\\debug\\libdeve_mobile.so" in jniLibs dir "E:\\gitclone\\Deve-Notebook\\apps\\mobile\\gen/android\\app/src/main/jniLibs/x86_64"
Info symlink at "E:\\gitclone\\Deve-Notebook\\apps\\mobile\\gen/android\\app/src/main/jniLibs/x86_64\\libdeve_mobile.so" points to "E:\\gitclone\\Deve-Notebook\\target\\x86_64-linux-android\\debug\\libdeve_mobile.so"
```

Emulator boot evidence:

```text
INFO | Boot completed in 70854 ms
mobile-android-emulator-install-startup-smoke-check: serial=emulator-5554 log=target/mobile-android-emulator-smoke/emulator.log
mobile-android-emulator-install-startup-smoke-check: ok
```

Non-blocking Kotlin daemon warning observed during Gradle build:

```text
this and base files have different roots: C:\Users\QQ\scoop\persist\rustup\.cargo\registry\src\...\ActivityCallback.kt and E:\gitclone\Deve-Notebook\apps\mobile\gen\android
Using fallback strategy: Compile without Kotlin daemon
```

The build recovered via fallback compilation and exited successfully.

## Previous Failure Resolved

The first emulator smoke attempt used the script default AVD location under `target/mobile-android-avd` on E: and failed before boot:

```text
Not enough space to create userdata partition. Available: 3219.66 MB ... need 7372.80 MB.
```

The successful run set `DEVE_MOBILE_ANDROID_AVD_HOME=/c/Users/QQ/.deve-mobile-android-avd`, where C: had sufficient free space.

## Skipped

- Signing and AAB/store readiness were not opened.
- Physical-device readiness was not opened.
- Android process runtime remained closed.
- Native authority writes remained closed.
- Web Git writer and server-backed Settings API remained closed.

## Result

Android shell-only package build and API 36 emulator install/startup smoke are locally closed at `ca00bd38`.

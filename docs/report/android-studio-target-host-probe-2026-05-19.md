# Android Studio Target-host Probe - 2026-05-19

本报告记录 Windows 主机上 Android Studio / Android SDK target-host 工具链探测、补齐与 Android shell package build gate 结果。`docs/plan/` 未修改。

## Scope

- Runtime head: `1b940222 Record Windows desktop package installer evidence`.
- Host: Microsoft Windows `10.0.26200.8457`.
- Target: `mobile-android` shell-only package/build evidence preparation.
- Non-goal: signing、store、physical-device readiness、Android process runtime、native authority writes、Web Git writer、server-backed Settings API。

## Toolchain

- Android Studio: scoop `android-studio 2025.3.4.7`, build `AI-253.32098.37.2534.15336583`.
- Android JBR: OpenJDK `21.0.10`, `javac 21.0.10`.
- Rust: `rustc 1.94.0`, `cargo 1.94.0`.
- Rust targets: `aarch64-linux-android`, `x86_64-linux-android`, `aarch64-pc-windows-msvc`, `x86_64-pc-windows-msvc`, `wasm32-unknown-unknown`.
- Node: `v24.15.0`.
- Trunk: `0.21.14`.
- Tauri CLI: `tauri-cli 2.11.1` from `target/native-tools/bin`.
- Android command-line tools: `sdkmanager 20.0` from scoop `android-clt 14742923`.
- Android SDK packages installed:
  - `platform-tools 37.0.0`
  - `emulator 36.5.11`
  - `platforms;android-36`
  - `build-tools;36.0.0`
  - `system-images;android-36;default;x86_64`
  - `ndk;28.2.13676358`

## Commands

Initial no-side-effect probe:

```bash
adb version
adb devices
where.exe bash
"C:\Users\QQ\scoop\apps\git\2.54.0\bin\bash.exe" -lc 'pwd; uname -s'
```

Target-host toolchain prep:

```bash
rustup target add aarch64-linux-android x86_64-linux-android
scoop install android-clt

sdkmanager "platform-tools" \
  "emulator" \
  "platforms;android-36" \
  "build-tools;36.0.0" \
  "system-images;android-36;default;x86_64" \
  "ndk;28.2.13676358"
```

Gate runs:

```bash
"C:\Users\QQ\scoop\apps\git\2.54.0\bin\bash.exe" -lc \
  'time ./scripts/check-native-track-boundary.sh'

DEVE_MOBILE_PACKAGE_TARGETS=android \
DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1 \
./scripts/check-mobile-platform-package-preflight.sh

DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1 \
DEVE_MOBILE_ANDROID_PACKAGE_TARGET=aarch64 \
DEVE_MOBILE_ANDROID_PACKAGE_APK=1 \
DEVE_MOBILE_ANDROID_PACKAGE_AAB=0 \
./scripts/check-mobile-android-shell-package-build.sh
```

The gate commands were run with temporary environment values:

```bash
ANDROID_HOME=/c/Users/QQ/scoop/apps/android-clt/current
ANDROID_SDK_ROOT=$ANDROID_HOME
ANDROID_NDK_HOME=$ANDROID_HOME/ndk/28.2.13676358
JAVA_HOME=/c/Users/QQ/scoop/apps/android-studio/current/jbr
PATH=/e/gitclone/Deve-Notebook/target/native-tools/bin:$JAVA_HOME/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH
```

## Passed

- `adb` available and reports `Android Debug Bridge version 1.0.41`, platform-tools `37.0.0`.
- No connected Android device or running emulator was present during probe.
- Windows-native Git Bash was located at `C:\Users\QQ\scoop\apps\git\2.54.0\bin\bash.exe`; default `bash` was WSL and too slow for these gates on the Windows-mounted worktree.
- `scripts/check-native-track-boundary.sh`: passed, `real 2m45.085s`.
- Android-only required platform preflight: passed.
- Host cargo checks inside preflight passed:
  - `cargo check --locked -p deve_mobile --no-default-features`
  - `cargo check --locked -p deve_mobile --features native-packaging`
  - `cargo test --locked -p deve_mobile --features native-packaging packaging -- --nocapture`
- Mobile packaging tests: `7 passed`.
- Android package build reached Rust release compilation and produced `target/aarch64-linux-android/release/libdeve_mobile.so`.

## Failed / Blocked

`scripts/check-mobile-android-shell-package-build.sh` failed after Rust compilation, when Tauri attempted to link the Android native library into `jniLibs`:

```text
Info symlinking lib "E:\\gitclone\\Deve-Notebook\\target\\aarch64-linux-android\\release\\libdeve_mobile.so" in jniLibs dir "E:\\gitclone\\Deve-Notebook\\apps\\mobile\\gen/android\\app/src/main/jniLibs/arm64-v8a"
Error failed to build Android app: Failed to create a symbolic link ...
Creation symbolic link is not allowed for this system.
```

Root cause: current Windows target host does not allow unprivileged symbolic link creation. Tauri Android build requires a symlink from the Rust `.so` into generated Android `jniLibs`. This is a host privilege / Developer Mode prerequisite, not a project code defect.

## Skipped

- `scripts/check-mobile-android-emulator-install-startup-smoke.sh` was not run because package build is blocked before APK creation.
- Android physical-device readiness, signing, store readiness, Android process runtime, native authority writes, Web Git writer, and server-backed Settings API remained closed.

## Next Action

Enable Windows Developer Mode or otherwise grant `SeCreateSymbolicLinkPrivilege` to the shell user, then rerun:

```bash
DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1 \
DEVE_MOBILE_ANDROID_PACKAGE_TARGET=aarch64 \
DEVE_MOBILE_ANDROID_PACKAGE_APK=1 \
DEVE_MOBILE_ANDROID_PACKAGE_AAB=0 \
./scripts/check-mobile-android-shell-package-build.sh
```

After package build passes, run emulator install/startup smoke with API 36 to match `compileSdk = 36`.

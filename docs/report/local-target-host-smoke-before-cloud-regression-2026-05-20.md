# Local Target-host Smoke Before Cloud Regression - 2026-05-20

本报告记录本机 Windows Desktop 与 Android target-host smoke。`docs/plan/` 未修改。

## Scope

- Baseline before local fixes: `1f4ed916 Add bilingual README and MIT license`.
- Target: Windows Desktop package/build/startup/native-session/installer smoke；Android shell APK package build 与 API 36 emulator install/startup smoke。
- Non-goal / kept closed: signing、store、physical-device readiness、native authority writes、Mobile process runtime、Android process runtime、Web Git writer、server-backed Settings API。
- Execution policy: 本机任务先跑完；Codex cloud full regression 不同时启动。

## Host / Toolchain

- Windows: `Microsoft Windows [Version 10.0.26200.8457]`.
- Rust: `rustc 1.94.0 (4a4ef493e 2026-03-02)`, host `x86_64-pc-windows-msvc`.
- Cargo: `cargo 1.94.0 (85eff7c80 2026-01-15)`.
- Node: `v24.15.0`.
- Trunk: `trunk 0.21.14`.
- Tauri CLI: `tauri-cli 2.11.1`.
- Android Studio: `AI-253.32098.37.2534.15336583`.
- Java/JBR: OpenJDK `21.0.10`.
- Android SDK tools: `sdkmanager 20.0`, `adb 37.0.0-14910828`, emulator `36.5.11.0`.
- Android SDK/NDK used: `platforms;android-36`, `system-images;android-36;default;x86_64`, `ndk;28.2.13676358`.
- Android AVD home: `C:\Users\QQ\.deve-mobile-android-avd`.

## Commands

Windows Desktop commands used Windows Git Bash with:

```bash
export PATH="$PWD/target/native-tools/bin:$PATH"
```

- `./scripts/check-desktop-package-preflight.sh`: passed after stopping concurrent background Cargo/rust-analyzer checks.
- `DEVE_DESKTOP_TARGET_HOSTS=windows DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1 ./scripts/check-desktop-target-host-preflight.sh`: passed.
- `DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis DEVE_DESKTOP_PACKAGE_NO_SIGN=1 ./scripts/check-desktop-platform-package-build.sh`: passed after fixing one `deve_cli` warning and rerunning.
- `DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis ./scripts/check-desktop-package-startup-smoke.sh`: passed.
- `DEVE_DESKTOP_NATIVE_SESSION_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis ./scripts/check-desktop-native-session-package-smoke.sh`: passed.
- `DEVE_DESKTOP_INSTALLER_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis ./scripts/check-desktop-installer-smoke.sh`: first run failed on stale MSI install registry; fixed and rerun passed.
- `./scripts/check-native-process-adapter-gate.sh`: passed.
- `./scripts/check-native-packaging-gate.sh`: passed.

Android commands used:

```bash
export ANDROID_HOME=/c/Users/QQ/scoop/apps/android-clt/current
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
export NDK_HOME="$ANDROID_NDK_HOME"
export JAVA_HOME=/c/Users/QQ/scoop/apps/android-studio/current/jbr
export PATH="$PWD/target/native-tools/bin:$JAVA_HOME/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
```

- `DEVE_MOBILE_ANDROID_PACKAGE_BUILD_REQUIRED=1 DEVE_MOBILE_ANDROID_PACKAGE_TARGET=aarch64 DEVE_MOBILE_ANDROID_PACKAGE_APK=1 DEVE_MOBILE_ANDROID_PACKAGE_AAB=0 ./scripts/check-mobile-android-shell-package-build.sh`: passed, exit code `0`.
- `DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED=1 DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL=36 DEVE_MOBILE_ANDROID_EMULATOR_SYSTEM_TARGET=default DEVE_MOBILE_ANDROID_EMULATOR_ARCH=x86_64 DEVE_MOBILE_ANDROID_AVD_HOME=/c/Users/QQ/.deve-mobile-android-avd DEVE_MOBILE_ANDROID_PACKAGE_TARGET=x86_64 ./scripts/check-mobile-android-emulator-install-startup-smoke.sh`: passed.

The emulator smoke exported temporary Bash wrappers for `sdkmanager.bat` and `avdmanager.bat`; no repository script was modified for that.

## Fixes Made

- `apps/cli/src/server/repo_scope/sync_bootstrap.rs`: changed an unused guarded error binding to `Err(_)`.
  - Targeted verification: `cargo check --locked -p deve_cli --bin deve_cli` passed.
  - Related verification: Windows Desktop package build reran and passed without the previous unused-variable warning.
- `scripts/check-desktop-installer-smoke.sh`: Windows installer smoke now snapshots and clears `HKCU\Software\deve\Deve Notebook` before MSI/NSIS installer checks, then restores the previous key state on cleanup.
  - Root cause: generated WiX `AppSearch` reads the previous install directory from that HKCU key and can override command-line `INSTALLDIR`, so repeated smoke runs could install MSI into a stale NSIS smoke path.
  - Targeted verification: `bash -n ./scripts/check-desktop-installer-smoke.sh` passed; required MSI/NSIS installer smoke passed twice after the fix; final registry check reported `registry key absent`.

## Passed Items

- Windows required target-host preflight detected target-host prerequisites as present.
- Desktop package build produced unsigned MSI and NSIS artifacts with `DEVE_DESKTOP_PACKAGE_NO_SIGN=1`.
- Desktop release startup smoke printed `desktop-startup-smoke: ok`.
- Desktop native-session package smoke printed `desktop-native-session-smoke: ok`.
- MSI install/startup/uninstall smoke passed after registry isolation.
- NSIS install/startup/uninstall smoke passed.
- Native process adapter gate and native packaging gate passed; process runtime and authority writes remained closed.
- Android `aarch64` release APK package build passed.
- Android API 36 `x86_64` emulator install/startup smoke passed.

## Failed / Resolved Items

- First Desktop preflight attempt timed out because unrelated background Cargo/rust-analyzer checks were already running against the same workspace target directory. Those background checks were stopped so this session could proceed without concurrent local work; rerun passed.
- First MSI installer smoke after the package rebuild failed with:
  - `MSI install completed but installed binary was not found under ...windows-msi...`
  - Log evidence showed `INSTALLDIR` was overwritten to an old `target/desktop-installer-smoke/windows-nsis...` path during `AppSearch`.
  - Fixed by registry isolation in `scripts/check-desktop-installer-smoke.sh`; rerun passed.

## Skipped / Kept Closed

- Signing was not run; Desktop package build used `DEVE_DESKTOP_PACKAGE_NO_SIGN=1`.
- Store readiness and physical-device readiness were not opened.
- Native authority writes remained closed.
- Mobile process runtime and Android process runtime remained closed.
- Web Git writer and server-backed Settings API remained closed.

## Artifacts

- `target/release/bundle/msi/Deve Notebook_0.0.1_x64_en-US.msi`: `29937664` bytes.
- `target/release/bundle/nsis/Deve Notebook_0.0.1_x64-setup.exe`: `20436526` bytes.
- `apps/mobile/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk`: `56440509` bytes.
- `apps/mobile/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`: `154855890` bytes.
- Android emulator log: `target/mobile-android-emulator-smoke/emulator.log`.

## Key Logs

```text
desktop-target-host-preflight-check: prerequisites present; run scripts/check-desktop-platform-package-build.sh on this target host to build packages
desktop-target-host-preflight-check: ok
Warn --no-sign flag detected: Signing will be skipped.
Built application at: E:\gitclone\Deve-Notebook\target\release\deve_desktop.exe
Finished 2 bundles at:
  E:\gitclone\Deve-Notebook\target\release\bundle\msi\Deve Notebook_0.0.1_x64_en-US.msi
  E:\gitclone\Deve-Notebook\target\release\bundle\nsis\Deve Notebook_0.0.1_x64-setup.exe
desktop-startup-smoke: ok
desktop-native-session-smoke: ok
desktop-installer-smoke-check: isolating Windows install registry key HKCU\Software\deve\Deve Notebook
desktop-installer-smoke-check: ok
native-process-adapter-gate-check: ok
native-packaging-gate-check: ok
mobile-platform-package-preflight-check: prerequisites present; Android shell package build is allowed only through scripts/check-mobile-android-shell-package-build.sh
mobile-android-shell-package-build-check: ok
Performing Streamed Install
Success
mobile-android-install-startup-smoke-check: app_id=dev.deve.notebook.mobile pid=2512
mobile-android-install-startup-smoke-check: serial=emulator-5554
mobile-android-install-startup-smoke-check: ok
mobile-android-emulator-install-startup-smoke-check: ok
```

## Next

本机 target-host evidence 已可交给 Codex cloud 跑 full regression gate refresh。云端范围应继续保持 signing、store、physical-device readiness、native authority writes、Mobile process runtime、Android process runtime、Web Git writer 与 server-backed Settings API 关闭。

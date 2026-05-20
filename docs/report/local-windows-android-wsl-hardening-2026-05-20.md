# Local Windows / Android Studio / WSL Hardening - 2026-05-20

本报告记录根据 Claude Code audit 报告推进的本机 Windows、Android Studio/SDK、WSL Ubuntu evidence 路线收束。`docs/plan/` 未修改。

## Scope

- Baseline: `be7390dc Record failed Apple target-host diagnostic`.
- Inputs reviewed: `docs/report/code-audit-2026-05-20.md`, `docs/report/docs-audit-2026-05-20.md`, `docs/report/plan-audit-2026-05-20.md`, `docs/report/apple-target-host-evidence-refresh-2026-05-20.md`, `docs/report/windows-desktop-smoke-2026-05-20.md`, `docs/report/next-tasks.md`.
- Target: 本机 Windows Desktop preflight continuity、Android Studio/SDK target-host tool discovery、WSL Ubuntu route hygiene.
- Non-goal / kept closed: signing、store、physical-device readiness、native authority writes、Mobile process runtime、Android process runtime、Web Git writer、server-backed Settings API。
- Apple scope: 未尝试用普通 Linux cloud 或 Windows/WSL 声明 Apple target-host closure；Apple evidence 仍需要真实 macOS target host。

## Host / Toolchain

- Windows: `Microsoft Windows [Version 10.0.26200.8457]`.
- Windows Rust: `rustc 1.94.0 (4a4ef493e 2026-03-02)`, host `x86_64-pc-windows-msvc`.
- Windows Cargo: `cargo 1.94.0 (85eff7c80 2026-01-15)`.
- Windows Node: `v24.15.0`.
- Windows Trunk: `trunk 0.21.14`.
- Windows Tauri CLI: `tauri-cli 2.11.2`.
- Android SDK root: `C:\Users\QQ\scoop\apps\android-clt\current`.
- Android Studio JBR: `openjdk 21.0.10`, discovered at `/c/Users/QQ/scoop/apps/android-studio/current/jbr`.
- Android adb: `Android Debug Bridge version 1.0.41`, `Version 37.0.0-14910828`.
- Android emulator: `36.5.11.0 (build_id 15261927)`.
- Android current target state before required smoke: no adb device listed by default; SDK emulator exists outside PATH.
- WSL Ubuntu: Linux `6.6.114.1-microsoft-standard-WSL2`, `rustc 1.92.0`, `cargo 1.92.0`, Node `v18.19.1`, Trunk `0.21.14`, Tauri CLI `2.11.1`, `glib-2.0 2.80.0`.
- WSL repo state: `~/gitclone/Deve-Notebook` was still at `eb30caff Record iOS closure full regression gate`; current-head WSL gates must not be declared from that clone until it is synced.
- WSL mounts: `/mnt/e` is read-only; `/mnt/egitclone` is writable. Prefer synced `~/gitclone/Deve-Notebook` for WSL gates to avoid Windows/WSL target directory contention.

## Fixed Bugs / Hardening

- Added `scripts/lib/android-tools.sh` for Android SDK tool discovery from PATH, `ANDROID_HOME`, or `ANDROID_SDK_ROOT`.
- Android helper now discovers SDK-local `adb`, `emulator`, `sdkmanager`, and `avdmanager` even when they are not on PATH.
- Android helper detects stale shell Java and falls back to Android Studio JBR 17+; this fixes `sdkmanager`/`avdmanager` and `keytool` diagnostics when Oracle Java 8 appears earlier on PATH.
- Hardened the helper against unset `USER` under `set -u`.
- Updated Android package/emulator/install/release preflight scripts to use the helper instead of assuming PATH-only Android tools.
- Updated `scripts/AGENTS.md` and `docs/dev-runbook.md` to document the shared Android helper and local Android Studio/JBR fallback.
- Terminated stale WSL build processes left by an interrupted `check-desktop-package-preflight.sh` run against `/mnt/egitclone/Deve-Notebook/target`.
- Updated `docs/report/next-tasks.md` so Apple evidence is no longer assigned to ordinary Linux cloud; it now explicitly requires real macOS target-host capacity.

## Commands / Results

```bash
git log -1 --oneline
# be7390dc Record failed Apple target-host diagnostic

C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc 'bash -n scripts/lib/android-tools.sh scripts/check-mobile-platform-package-preflight.sh scripts/check-mobile-android-emulator-install-startup-smoke.sh scripts/check-mobile-android-install-startup-smoke.sh scripts/check-mobile-android-release-preflight.sh'
# passed

C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc 'DEVE_DESKTOP_TARGET_HOSTS=windows DEVE_DESKTOP_PACKAGE_NO_SIGN=1 ./scripts/check-desktop-target-host-preflight.sh'
# passed

C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc 'DEVE_MOBILE_PACKAGE_TARGETS=android DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1 DEVE_MOBILE_PACKAGE_HOST_NATIVE_PACKAGING_CHECK=0 ./scripts/check-mobile-platform-package-preflight.sh'
# passed

C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc 'DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL=36 DEVE_MOBILE_ANDROID_AVD_HOME=/c/Users/QQ/.deve-mobile-android-avd ./scripts/check-mobile-android-emulator-install-startup-smoke.sh'
# diagnostic pass

C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc './scripts/check-mobile-android-install-startup-smoke.sh'
# diagnostic pass

C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc './scripts/check-mobile-android-release-preflight.sh'
# diagnostic pass; signing and physical-device readiness remain closed

C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc 'DEVE_MOBILE_ANDROID_EMULATOR_INSTALL_STARTUP_SMOKE_REQUIRED=1 DEVE_MOBILE_ANDROID_EMULATOR_API_LEVEL=36 DEVE_MOBILE_ANDROID_AVD_HOME=/c/Users/QQ/.deve-mobile-android-avd ./scripts/check-mobile-android-emulator-install-startup-smoke.sh'
# required pass

C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc './scripts/check-dev-runbook-baseline.sh'
# passed

C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc './scripts/check-mobile-baseline.sh'
# passed
```

## Passed Items

- Windows Desktop target-host preflight passed on current local Windows host; package prerequisites remain present.
- Desktop native packaging shell tests passed during the preflight run: `5 passed`.
- Android platform package preflight passed in required Android-only scope with host native packaging check disabled for the scoped validation.
- Android emulator install/startup smoke passed in required mode with API 36 and `ANDROID_AVD_HOME=/c/Users/QQ/.deve-mobile-android-avd`.
- Android release preflight passed in diagnostic mode while continuing to report missing signing material and physical device as closed-scope prerequisites.
- Android release preflight also reran mobile native-packaging checks and packaging tests: `7 passed`.
- Dev runbook baseline and mobile baseline passed after documentation/script updates.
- WSL Ubuntu toolchain probe succeeded; `glib-2.0` is present in WSL.

## Failed / Resolved Items

- Direct Android SDK diagnostics previously hit Java 8 on PATH (`sdkmanager` requires Java 17+). Resolved by Android Studio JBR fallback.
- SDK `emulator.exe` existed under `ANDROID_HOME\emulator` but was not on PATH. Resolved by SDK-local tool discovery.
- WSL had stale build processes from an interrupted mounted-path preflight. Resolved by terminating the parent `bash`/`cargo` processes and confirming no matching `cargo`/`rustc` processes remained.

## Skipped / Kept Closed

- Desktop package build/startup/installer smoke was not rerun in full in this batch because Desktop product code was not changed; the previous Windows Desktop smoke report remains the package/startup/installer evidence reference.
- WSL current-head gates were not declared because the native WSL clone is still at `eb30caff`, not current `HEAD`.
- Android signing and physical-device readiness were not opened.
- Android process runtime and Mobile process runtime remained closed.
- Native authority writes, Web Git writer, server-backed Settings API, signing, and store readiness remained closed.
- Apple macOS/iOS evidence remains blocked until a real macOS target host is available.

## Key Logs

```text
desktop-target-host-preflight-check: host_os=MSYS_NT-10.0-26200
desktop-target-host-preflight-check: targets=windows
desktop-target-host-preflight-check: prerequisites present; run scripts/check-desktop-platform-package-build.sh on this target host to build packages
desktop-target-host-preflight-check: ok

mobile-platform-package-preflight-check: host_os=MSYS_NT-10.0-26200
mobile-platform-package-preflight-check: targets=android
mobile-platform-package-preflight-check: prerequisites present; Android shell package build is allowed only through scripts/check-mobile-android-shell-package-build.sh
mobile-platform-package-preflight-check: ok

mobile-android-emulator-install-startup-smoke-check: api=36 target=default arch=x86_64 avd=deve-mobile-smoke-api36-default-x86_64
mobile-android-emulator-install-startup-smoke-check: ok

mobile-android-release-preflight-check: signed release and physical-device smoke not executed
mobile-android-release-preflight-check: missing signing keystore via DEVE_ANDROID_KEYSTORE_PATH or DEVE_ANDROID_KEYSTORE_BASE64
mobile-android-release-preflight-check: missing physical-device physical Android device (non-emulator adb target)
mobile-android-release-preflight-check: ok
```

## Next

1. Commit and push this local hardening batch before using WSL `~/gitclone/Deve-Notebook`.
2. Sync WSL native clone to the new commit, then run Linux/WSL gates from the native clone or use a separate `CARGO_TARGET_DIR` if testing through `/mnt/egitclone`.
3. Keep Apple target-host evidence separate and run it only on a real macOS host.

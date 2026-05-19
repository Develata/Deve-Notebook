# Windows Desktop Smoke - 2026-05-18

本报告记录 Windows Desktop target-host smoke。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/08_ui_design_02_desktop.md` 的 Current Native Boundary、native-packaging、process adapter gate。
- Target: Windows Desktop package/startup/installer smoke 与 native gate 复核。
- Non-goal: signing、store、physical-device readiness、native authority writes、Mobile process runtime、Android process runtime、Web Git writer、server-backed Settings API。
- Baseline: `eb30caff Record iOS closure full regression gate`。

## Host

- Windows: `Microsoft Windows [Version 10.0.26200.8457]`。
- Windows Rust: `cargo 1.94.0 (85eff7c80 2026-01-15)` / `rustc 1.94.0 (4a4ef493e 2026-03-02)`, host `x86_64-pc-windows-msvc`。
- Windows Node: `v24.15.0`。
- Windows Trunk: `trunk 0.21.14`。
- Windows Tauri CLI: `cargo tauri --version` failed, `error: no such command: tauri`。
- Windows Git Bash: `MINGW64_NT-10.0-26200`, `OS=Windows_NT`。
- Default `bash`: `C:\Windows\System32\bash.exe`, WSL2 `Linux LAPTOP-H6JEOCF0 6.6.114.1-microsoft-standard-WSL2`。
- WSL login shell tools: `cargo 1.92.0`, `rustc 1.92.0`, `node v18.19.1`, `trunk 0.21.14`, `tauri-cli 2.11.1`。

## Baseline Commands

- `git pull origin main`: failed because `origin` is not configured.
- `git remote -v`: configured remote is `Deve-Notebook https://github.com/Develata/Deve-Notebook.git`.
- `git pull Deve-Notebook main`: already up to date.
- `git status --short`: clean before smoke edits.
- `git log -1 --oneline`: `eb30caff Record iOS closure full regression gate`.

## Verification

- `bash scripts/check-desktop-package-preflight.sh`: initial direct `bash` attempt failed in WSL non-login PATH with `cargo: command not found`; Windows Git Bash run passed.
- `DEVE_DESKTOP_TARGET_HOSTS=windows bash scripts/check-desktop-target-host-preflight.sh`: passed diagnostic path; package readiness not claimed.
- `DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis bash scripts/check-desktop-platform-package-build.sh`: passed diagnostic path; package build skipped because `cargo tauri CLI` is missing.
- `DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis bash scripts/check-desktop-package-startup-smoke.sh`: passed diagnostic path; startup smoke skipped because package/release artifacts are missing.
- `DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis bash scripts/check-desktop-installer-smoke.sh`: passed diagnostic path; installer smoke skipped because MSI/NSIS artifacts are missing.
- `bash scripts/check-native-process-adapter-gate.sh`: passed.
- `bash scripts/check-native-packaging-gate.sh`: passed after targeted fixes; native-session package smoke skipped because no release package/sidecar artifacts exist.

## Passed Items

- Desktop package preflight passed on Windows Git Bash:
  - `check-native-track-boundary.sh`: ok.
  - `cargo check --locked -p deve_desktop --no-default-features`: ok.
  - `cargo test --locked -p deve_desktop --no-default-features -- --nocapture`: 21 passed.
  - `cargo check --locked -p deve_desktop --features native-packaging`: ok.
  - `cargo test --locked -p deve_desktop --features native-packaging menu_tray -- --nocapture`: 3 passed.
  - `cargo test --locked -p deve_desktop --features native-packaging packaging -- --nocapture`: 5 passed.
- Windows target-host preflight compiled and tested Desktop native-packaging packaging surface: 5 passed.
- Native process adapter gate passed:
  - `deve_core native_adapter::process_test`: 12 passed.
  - Desktop/Mobile default process adapter tests passed.
  - `deve_cli native_session`: 7 passed.
  - Desktop `service_entrypoint`: 4 passed.
  - Desktop `service_bootstrap`: 10 passed.
  - Desktop/Mobile `process_observation`: 3 + 3 passed.
- Native packaging gate passed:
  - Desktop `process_runtime`: 15 passed.
  - Desktop `service_entrypoint`: 4 passed.
  - Desktop `service_bootstrap`: 10 passed.
  - Desktop `tauri_bootstrap`: 7 passed.
  - Desktop `menu_tray`: 3 passed.
  - Desktop `packaging`: 5 passed.
  - Mobile `packaging`: 7 passed.
  - `deve_cli native_session`: 7 passed.

## Failed Items Fixed

- Windows Git Bash exposed mixed path separators in native gate `rg` output, e.g. `E:/gitclone/.../apps/desktop/src\process_runtime.rs`, causing legal Desktop post-gate runtime spike files to be rejected. Fixed by normalizing search-result paths in `scripts/check-native-track-boundary.sh` and `scripts/check-native-process-adapter-gate.sh`.
- Windows Git Bash used Windows `rg` with MSYS `/e/...` paths, causing fixed-string checks to falsely report missing strings. Fixed by using `grep` for file-path searches on Windows bash hosts while retaining `rg` for stdin searches.
- Windows `deve_mobile --features native-packaging` failed because Tauri build required platform icons for Windows resource generation. Fixed by adding `apps/mobile/icons/icon.ico`, `apps/mobile/icons/icon.icns`, updating `apps/mobile/tauri.conf.json`, and extending the mobile packaging test/native boundary guard.

## Skipped Items

- `cargo tauri build` was not run: Windows `cargo tauri CLI` is not installed.
- Windows target-host package build remained diagnostic-only; missing `cargo tauri CLI`, WiX Toolset, NSIS `makensis`, and `aarch64-pc-windows-msvc`.
- Startup smoke was skipped because `target/release/deve_desktop.exe`, MSI, and NSIS artifacts are absent.
- Installer smoke was diagnostic-only because MSI/NSIS artifacts are absent.
- Native-session package smoke inside `check-native-packaging-gate.sh` skipped because `target/release/deve_desktop.exe` and `target/release/deve_cli.exe` are absent.

## Key Logs

```text
 desktop-package-preflight-check: ok
 desktop-target-host-preflight-check: host_os=MINGW64_NT-10.0-26200
 desktop-target-host-preflight-check: missing cargo tauri CLI
 desktop-target-host-preflight-check: missing WiX Toolset
 desktop-target-host-preflight-check: missing NSIS makensis
 desktop-target-host-preflight-check: missing rust target aarch64-pc-windows-msvc
 desktop-platform-package-build-check: missing cargo tauri CLI
 desktop-package-startup-smoke-check: missing Windows MSI target/release/bundle/msi/*.msi
 desktop-package-startup-smoke-check: missing Windows NSIS target/release/bundle/nsis/*.exe
 desktop-package-startup-smoke-check: missing Windows release binary target/release/deve_desktop.exe
 desktop-installer-smoke-check: missing Windows MSI target/release/bundle/msi/*.msi
 desktop-installer-smoke-check: missing Windows NSIS target/release/bundle/nsis/*.exe
 native-process-adapter-gate-check: ok
 desktop-native-session-package-smoke-check: missing Windows release binary target/release/deve_desktop.exe
 desktop-native-session-package-smoke-check: missing Windows sidecar target/release/deve_cli.exe
 native-packaging-gate-check: ok
```

## Decision

Windows Desktop package/startup/installer smoke did not produce installer evidence because the target host lacks Tauri CLI and installer toolchain/artifacts. The Desktop/native gates are otherwise closed for this scope: process runtime and native authority writes remain closed, Desktop package preflight passed, and native packaging/process adapter gates passed after Windows-host guard fixes.

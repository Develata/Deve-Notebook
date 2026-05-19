# Windows Desktop Package/Installer Smoke - 2026-05-19

本报告记录 Windows Desktop target-host 工具链补齐后的 package/build/startup/installer smoke。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/08_ui_design_02_desktop.md` 的 Current Native Boundary、native-packaging、process adapter gate。
- Target: Windows Desktop target-host preflight、MSI/NSIS package build、release startup smoke、native-session package smoke、installer install/uninstall smoke。
- Non-goal: signing、store、physical-device readiness、native authority writes、Mobile process runtime、Android process runtime、Web Git writer、server-backed Settings API。
- Baseline: `d88ce6cc Record Windows desktop smoke evidence`。

## Host

- Windows: `Microsoft Windows [Version 10.0.26200.8457]`。
- Rust: `cargo 1.94.0 (85eff7c80 2026-01-15)` / `rustc 1.94.0 (4a4ef493e 2026-03-02)`, host `x86_64-pc-windows-msvc`。
- Rust targets installed: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`, `wasm32-unknown-unknown`。
- Node: `v24.15.0`。
- Trunk: `trunk 0.21.14`。
- Tauri CLI: `tauri-cli 2.11.1` from `target/native-tools/bin/cargo-tauri.exe`。
- WiX: `7.0.0+b8977d6`; package build also used Tauri-managed WiX3 bundle cache.
- NSIS: `makensis v3.12`; package build also used Tauri-managed NSIS bundle helper.
- .NET: `8.0.421`。
- Windows Git Bash: `MINGW64_NT-10.0-26200`。

## Toolchain Setup

- `git pull Deve-Notebook main`: already up to date.
- `rustup target add aarch64-pc-windows-msvc`: installed Windows ARM64 Rust std target.
- `DEVE_NATIVE_INSTALL_TRUNK=0 DEVE_NATIVE_INSTALL_TAURI_CLI=1 scripts/install-native-target-host-tools.sh`: installed `cargo-tauri 2.11.1` into `target/native-tools/bin`.
- `scoop install wixtoolset nsis`: installed WiX Toolset 7.0.0 and NSIS 3.12.

## Verification

- `DEVE_DESKTOP_TARGET_HOSTS=windows DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1 scripts/check-desktop-target-host-preflight.sh`: passed.
- `DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis DEVE_DESKTOP_PACKAGE_NO_SIGN=1 scripts/check-desktop-platform-package-build.sh`: passed.
- `DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis scripts/check-desktop-package-startup-smoke.sh`: passed.
- `DEVE_DESKTOP_NATIVE_SESSION_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis scripts/check-desktop-native-session-package-smoke.sh`: passed.
- `DEVE_DESKTOP_INSTALLER_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis scripts/check-desktop-installer-smoke.sh`: passed.

All Windows commands were run through Windows Git Bash with:

```bash
export PATH="$PWD/target/native-tools/bin:$PATH"
```

## Artifacts

- `target/release/deve_desktop.exe`
- `target/release/deve_cli.exe`
- `target/release/bundle/msi/Deve Notebook_0.0.1_x64_en-US.msi` (`29937664` bytes)
- `target/release/bundle/nsis/Deve Notebook_0.0.1_x64-setup.exe` (`20441412` bytes)

## Passed Items

- Required Windows target-host preflight detected all prerequisites as present.
- Desktop package preflight passed before package build.
- `cargo tauri build --ci --features native-packaging --no-sign --bundles msi,nsis` produced both MSI and NSIS artifacts.
- Release startup smoke printed `desktop-startup-smoke: ok`.
- Native-session package smoke started the packaged app with packaged `deve_cli` sidecar and printed `desktop-native-session-smoke: ok`.
- MSI install/uninstall smoke completed under `target/desktop-installer-smoke/...` and startup probe passed after install.
- NSIS install/uninstall smoke completed under `target/desktop-installer-smoke/...` and startup probe passed after install.

## Failed Items

- No target-host smoke failure remained after toolchain setup.

## Skipped / Kept Closed

- Signing was explicitly not run: package build used `DEVE_DESKTOP_PACKAGE_NO_SIGN=1`.
- Store, physical-device readiness, native authority writes, Mobile process runtime, Android process runtime, Web Git writer, and server-backed Settings API remained closed.

## Key Logs

```text
desktop-target-host-preflight-check: prerequisites present; run scripts/check-desktop-platform-package-build.sh on this target host to build packages
desktop-target-host-preflight-check: ok
desktop-platform-package-build-check: prepared deve_cli sidecar apps/desktop/binaries/deve_cli-x86_64-pc-windows-msvc.exe
Built application at: E:\gitclone\Deve-Notebook\target\release\deve_desktop.exe
Finished 2 bundles at:
  E:\gitclone\Deve-Notebook\target\release\bundle\msi\Deve Notebook_0.0.1_x64_en-US.msi
  E:\gitclone\Deve-Notebook\target\release\bundle\nsis\Deve Notebook_0.0.1_x64-setup.exe
desktop-startup-smoke: ok
desktop-package-startup-smoke-check: ok
desktop-native-session-smoke: ok
desktop-native-session-package-smoke-check: ok
desktop-installer-smoke-check: host_os=MINGW64_NT-10.0-26200
desktop-startup-smoke: ok
desktop-startup-smoke: ok
desktop-installer-smoke-check: ok
```

## Follow-up

- `apps/desktop/binaries/` is a transient Tauri sidecar staging directory created by `scripts/check-desktop-platform-package-build.sh`; it is now ignored to prevent release sidecar binaries from appearing as untracked source changes after target-host package builds.
- Future Windows evidence refreshes can start from required target-host preflight and the four package/startup/native-session/installer smoke scripts above, while keeping signing and native authority writes closed unless explicitly opened.

# Windows Desktop Smoke - 2026-05-20

本报告记录本机 Windows Desktop target-host smoke 与本轮 Windows 回归修复证据。`docs/plan/` 未修改。

## Scope

- Baseline: `a0f99761 Record local mainline cloud apple split`.
- Target: Windows Desktop package/build/startup/installer smoke；同时确认本轮 Windows mainline regression 修复后的 Rust gate。
- Execution host: 本机 Windows / Git Bash target-host。
- Non-goal / kept closed: signing、store、physical-device readiness、native authority writes、Mobile process runtime、Android process runtime、Web Git writer、server-backed Settings API。
- Web scope: 未重建 Web source；Desktop package build 使用仓库既有 `apps/web/dist`。

## Host / Toolchain

- Windows: `Microsoft Windows [Version 10.0.26200.8457]`；`Get-ComputerInfo` reported `Microsoft Windows 11 家庭中文版`, `OsVersion 10.0.26200`, `OsBuildNumber 26200`.
- Rust: `rustc 1.94.0 (4a4ef493e 2026-03-02)`, host `x86_64-pc-windows-msvc`.
- Node: `v24.15.0`.
- Trunk: `trunk 0.21.14`.
- Tauri CLI: `tauri-cli 2.11.2`.
- Windows installer toolchain: WiX and NSIS were present on PATH; `cargo tauri build` produced MSI and NSIS bundles.

## Commands

Windows Desktop commands used Git Bash:

```bash
C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc "./scripts/check-desktop-package-preflight.sh"
C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc "DEVE_DESKTOP_TARGET_HOSTS=windows DEVE_DESKTOP_PACKAGE_NO_SIGN=1 ./scripts/check-desktop-target-host-preflight.sh"
C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc "DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_NO_SIGN=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis ./scripts/check-desktop-platform-package-build.sh"
C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc "DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis ./scripts/check-desktop-package-startup-smoke.sh"
C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc "DEVE_DESKTOP_INSTALLER_SMOKE_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis ./scripts/check-desktop-installer-smoke.sh"
C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc "./scripts/check-native-process-adapter-gate.sh"
C:\Users\QQ\scoop\apps\git\2.54.0\usr\bin\bash.exe -lc "./scripts/check-native-packaging-gate.sh"
```

Rust verification run in this session:

```bash
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Targeted verification run for the Windows fixes included:

```bash
cargo test --locked -p deve_core --lib ledger::database_cache::tests::reusable_cached_database_reuses_locked_same_file_on_windows -- --nocapture
cargo test --locked -p deve_cli --bin deve_cli commands::repair::restore::tests::find_loading_corruption_fails_closed_on_unreadable_workspace_target -- --nocapture
cargo test --locked -p deve_cli --bin deve_cli server::agent_bridge::policy::tests -- --nocapture
cargo test --locked -p deve_core --test shadow_listing_test
cargo test --locked -p deve_core --test source_control_commit_apply_error_test
cargo test --locked -p deve_core --test watcher_create_modify_delete -- --nocapture
```

## Passed Items

- Desktop package preflight passed: no-default-features shell tests, native-packaging check/tests, menu/tray tests and packaging tests.
- Windows target-host preflight passed and reported prerequisites present.
- Desktop platform package build passed with `DEVE_DESKTOP_PACKAGE_NO_SIGN=1`.
- Desktop startup smoke passed.
- MSI installer smoke passed: silent install, startup smoke, silent uninstall.
- NSIS installer smoke passed: silent install, startup smoke, silent uninstall.
- Native process adapter gate passed while real runtime authority remained closed by policy.
- Native packaging gate passed, including desktop native-session package smoke.
- Full Rust test suite and locked all-features clippy passed after the Windows fixes.

## Fixed Bugs

- Windows `redb` cache reuse: active `redb` handles can lock the database file, so stale same-file cache validation could fail by trying to reread the file header. The Windows path now reuses the open handle when file identity is unchanged; non-Windows still validates the `redb` magic.
- Repair restore unreadable target: directory targets now fail with a stable `Workspace target is a directory` error instead of depending on platform-specific `read_to_string` wording.
- Agent bridge policy tests: missing executable fixtures now use platform-correct absolute temp paths instead of Unix-only `/definitely/...`.
- Shadow listing corruption test: the already-open corrupted `.redb` overwrite scenario is Unix-only because Windows correctly blocks overwriting the locked file.
- Source-control staged file errors: error context now includes both repo-relative path and physical disk path, so Windows backslash paths do not hide the logical target.
- Watcher root matching: watcher roots are canonicalized before backend registration so Windows notify event paths and dispatch roots share the same prefix form.
- Windows clippy hygiene: Unix-only imports/tests are now `#[cfg(unix)]` gated where the behavior depends on Unix permissions or symlink semantics.

## Failed / Resolved Items

- Direct non-login Bash invocation could not run the package preflight correctly because expected Git Bash coreutils were not on PATH. The gate was rerun with `bash.exe -lc`, matching the target-host shell environment; no code workaround was made.
- The sandbox could not spawn Git Bash directly (`CreateProcessAsUserW failed: 1312`). Commands were rerun with approved escalation so the target-host scripts could execute normally.
- No unresolved Desktop package/build/startup/installer failure remains in this run.

## Skipped / Kept Closed

- Signing was not run; package build emitted `Warn --no-sign flag detected: Signing will be skipped.`
- Store readiness and physical-device readiness were not opened.
- Native authority writes remained closed.
- Mobile process runtime and Android process runtime remained closed.
- Web Git writer and server-backed Settings API remained closed.
- Web release/source rebuild was not run in this Desktop target-host session.

## Artifacts

- `target/release/bundle/msi/Deve Notebook_0.0.1_x64_en-US.msi`: `29962240` bytes.
- `target/release/bundle/nsis/Deve Notebook_0.0.1_x64-setup.exe`: `20467752` bytes.

## Key Logs

```text
desktop-package-preflight-check: ok
desktop-target-host-preflight-check: host_os=MSYS_NT-10.0-26200
desktop-target-host-preflight-check: prerequisites present; run scripts/check-desktop-platform-package-build.sh on this target host to build packages
desktop-target-host-preflight-check: ok
Warn --no-sign flag detected: Signing will be skipped.
Built application at: E:\gitclone\Deve-Notebook\target\release\deve_desktop.exe
Finished 2 bundles at:
  E:\gitclone\Deve-Notebook\target\release\bundle\msi\Deve Notebook_0.0.1_x64_en-US.msi
  E:\gitclone\Deve-Notebook\target\release\bundle\nsis\Deve Notebook_0.0.1_x64-setup.exe
desktop-startup-smoke: ok
desktop-package-startup-smoke-check: ok
desktop-installer-smoke-check: isolating Windows install registry key HKCU\Software\deve\Deve Notebook
desktop-installer-smoke-check: ok
native-process-adapter-gate-check: ok
desktop-native-session-smoke: ok
desktop-native-session-package-smoke-check: ok
native-packaging-gate-check: ok
```

## Next

本机已重新闭合 Windows Desktop target-host package/build/startup/installer evidence。下一步先做 post-regression work selection；Codex Cloud 只在需要 macOS / iOS target-host evidence 时使用，不再承担常规主线 full regression。

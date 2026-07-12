# Native Target-host Evidence - Desktop Windows - 2026-07-12

Target: Desktop Windows

Workflow run: N/A - local Windows target-host execution at Git HEAD `7cc9b9a8f9dc747eeea854b485326920d0d7f912`

Host OS: Microsoft Windows 11 Home China 10.0.26200 build 26200, x86_64, 12th Gen Intel Core i7-12700H

Tool versions:

- rustc: `rustc 1.92.0 (ded5c06cf 2025-12-08)` (`x86_64-pc-windows-msvc`)
- cargo: `cargo 1.92.0 (344c4567c 2025-10-21)`
- cargo tauri: `tauri-cli 2.11.2`
- node: `v24.18.0`
- npm: `11.16.0`
- Git Bash: `GNU bash 5.3.15(1)-release (x86_64-pc-cygwin)`
- PowerShell: `7.6.3`
- platform toolchain: WiX `7.0.0+b8977d6`; NSIS `v3.12`; Windows MSVC host target; MSI and NSIS artifacts are intentionally unsigned public-preview evidence

Commands:

```bash
git pull --ff-only
git status --short --branch
git rev-parse HEAD
git log -3 --oneline

DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1 \
DEVE_DESKTOP_TARGET_HOSTS=windows \
scripts/check-desktop-target-host-preflight.sh

RUSTUP_TOOLCHAIN=1.92.0 \
DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 \
DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis \
scripts/check-desktop-platform-package-build.sh

DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED=1 \
DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis \
scripts/check-desktop-package-startup-smoke.sh

DEVE_DESKTOP_NATIVE_SESSION_SMOKE_REQUIRED=1 \
DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis \
scripts/check-desktop-native-session-package-smoke.sh

DEVE_DESKTOP_INSTALLER_SMOKE_REQUIRED=1 \
DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis \
scripts/check-desktop-installer-smoke.sh
```

The additional visible-window observation was launched from PowerShell with
`DEVE_DESKTOP_DATA_DIR` set to repo-owned temporary roots before starting
`E:\gitclone\Deve-Notebook\target\release\deve_desktop.exe`. The three roots
used while checking initial load timing and the rendered window were:

- `E:\gitclone\Deve-Notebook\target\desktop-manual-smoke.beb2460819e5454db9de01aaeae411c6`
- `E:\gitclone\Deve-Notebook\target\desktop-manual-smoke.3c254019549646e787dcf6c8bc825eb0`
- `E:\gitclone\Deve-Notebook\target\desktop-ui-smoke.b6e90a46094a43ebbe0d283c609e3bfc`

Command results:

- `desktop_preflight=passed`: required Windows preflight exited 0; native-track boundary, no-default/native-packaging checks, and 5 packaging tests passed.
- `process_gate=passed`: default no-Tauri authority remained closed; packaged LocalBackend used one controlled loopback `deve_cli` sidecar and removed it on exit.
- `invalid_startup_request=baseline-pass`: the deterministic startup-smoke baseline ran before the target-host probe; no invalid selector was used for package evidence.
- `invalid_installer_request=baseline-pass`: the deterministic installer-smoke baseline ran before the target-host installer probe; `exe` was not accepted as installer evidence.
- `package_build=passed`: `cargo tauri build --ci --features native-packaging --bundles msi,nsis` produced both requested bundles from this checkout.
- `startup_smoke=passed`: the packaged Desktop binary emitted `desktop-startup-smoke: ok` and exited 0.
- `native_session_smoke=passed`: the sibling `deve_cli.exe` started `serve --native-loopback`, `/api/auth/status` and native cookie handoff passed, and the temporary service stopped before success.
- `installer_smoke=passed`: MSI and NSIS each completed silent install, installed-binary startup probe, installer-specific uninstall, and post-uninstall absence checks.
- Manual window observation: after a 15-second readiness wait the release binary rendered the non-blank Deve-Note dashboard with `native-main`, `embedded-frontend`, `production`, `repo healthy`, and ready indicators. The first immediate screenshot was blank while WebView was still loading and was not treated as a product failure. The backend ledger/projection roots were isolated, but the visible Tauri/WebView2 launch used the existing app-private `C:\Users\QQ\AppData\Local\dev.deve.notebook\EBWebView` profile and modified cache/profile files there.
- Settings focus trap/restore manual interaction was not retained as evidence because host-window focus was not deterministic; automated UI focus contracts remain separate evidence.

Artifact paths:

- MSI: `E:\gitclone\Deve-Notebook\target\release\bundle\msi\Deve Notebook_0.1.0_x64_en-US.msi`; 29,220,864 bytes; SHA-256 `314F2DEC427C9C77F9FD9C27B46C929AEB79A189C5265ABA43853044323F61EB`; UTC mtime `2026-07-12T07:11:50.7640000Z`.
- NSIS: `E:\gitclone\Deve-Notebook\target\release\bundle\nsis\Deve Notebook_0.1.0_x64-setup.exe`; 20,680,954 bytes; SHA-256 `A6E1103A543CFDD1B4956246EAB3627E12ED1F3069EA39620895B74EE90A8E27`; UTC mtime `2026-07-12T07:12:51.6378704Z`.

Install result: passed - MSI per-user silent install/uninstall and NSIS silent install/uninstall completed under repo-owned temporary directories; no matching uninstall registry entry, installed executable, installer process, or smoke directory remained.

Startup result: passed - package startup, native-session handoff, and installed-binary probes succeeded. A normal visible LocalBackend launch exposed a healthy `native-main` node-role response from a random loopback port and rendered the dashboard after readiness.

Cleanup result:

- Remaining `deve_desktop` / `deve_cli` processes: none.
- Remaining smoke listeners: none.
- Remaining native-session, installer, or manual UI smoke directories: none.
- Real user app-data touched: yes. The visible manual launch modified WebView2 cache/profile files under `C:\Users\QQ\AppData\Local\dev.deve.notebook\EBWebView` between `2026-07-12T07:16:04Z` and `2026-07-12T07:20:35Z`. No attempt was made to delete or roll back this pre-existing user profile. The temporary `DEVE_DESKTOP_DATA_DIR` roots prove that notebook ledger/projection authority data for the manual launches stayed under repo-owned `target/`; they do not prove that the WebView profile was isolated.
- Worktree after target-host execution: only this evidence report is added; package outputs remain ignored under `target/`.

Process runtime boundary: default no-Tauri closed; Desktop LocalBackend controlled child-process; Mobile child-process closed

Native authority writes: closed

Conclusion: diagnostic-only - Windows x64 unsigned MSI/NSIS package, packaged startup, native-session, and installer install/uninstall gates passed on this target host, but the handoff's no-real-user-app-data-touch completion condition was not met by the additional visible-window observation because WebView2 used the existing app-private profile. This does not prove signing, Microsoft Store distribution, ARM64 Windows, macOS, Linux native release, or physical-device readiness.

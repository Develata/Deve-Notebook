# Desktop macOS/Windows Target-host Preflight

Date: 2026-05-14

## Scope

Added and ran a Desktop macOS/Windows target-host prerequisite preflight.

No macOS or Windows package was claimed on the Linux/WSL host.

## Result

- New script: `scripts/check-desktop-target-host-preflight.sh`.
- Default Linux/WSL run exits successfully but reports missing target hosts:
  - `macOS target-host requires Darwin`
  - `Windows target-host requires Windows`
- Required mode is available through `DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1`.
- Target selection is available through `DEVE_DESKTOP_TARGET_HOSTS=macos` or `windows`.
- Desktop shell/package boundary checks still pass before target-host diagnostics.
- The script does not run `cargo tauri build`; actual package build remains delegated to `scripts/check-desktop-platform-package-build.sh` on the target host.

## Target-host Requirements Captured

- macOS: Darwin host, `xcodebuild`, `xcrun`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `APPLE_SIGNING_IDENTITY`, `APPLE_PROVIDER_SHORT_NAME`.
- Windows: Windows host, PowerShell, MSVC `cl.exe`, WiX Toolset, NSIS `makensis`, `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`.

## Verification

- `scripts/check-desktop-target-host-preflight.sh`
- `scripts/check-release-baseline.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `git diff --check`

## Follow-up

- macOS/Windows package signing, installer generation, installation, and startup behavior remain target-host work.
- Current Linux/WSL-hosted execution can proceed to the Desktop process runtime gate decision only if macOS/Windows package verification is treated as target-host-blocked until those hosts are available.

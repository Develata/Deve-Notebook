# Desktop Tauri Runtime Entrypoint

Date: 2026-05-14

## Scope

- Active queue item: Desktop Tauri runtime entrypoint decision.
- Source of truth: `docs/plan/08_ui_design_02_desktop.md` and `docs/plan/14_tech_stack.md`.
- This batch does not modify `docs/plan/`.

## Changes

- Added `apps/desktop/build.rs`.
- Added `apps/desktop/src/main.rs`.
- Added `apps/desktop/src/tauri_entry.rs`.
- Added `apps/desktop/icons/icon.png` as the explicit Tauri package icon.
- Extended `apps/desktop/native-packaging` to include `tauri/wry`, because a real Tauri desktop runtime entrypoint requires the default Wry runtime.

## Runtime Boundary

- Default Desktop build remains no-Tauri.
- `native-packaging` starts only the Tauri window shell with menu and tray binding.
- Menu and tray actions map only to shell UI effects: show main window, toggle window visibility, or quit request.
- The entrypoint does not spawn the Deve backend service.
- The entrypoint does not write ledger, vault, source-control, search, Git, or `.notegit` authority.
- Platform package build/signing is not claimed on this host.

## Current Target-Host Gap

- `cargo tauri` CLI is not installed on the current host.
- A real package build still requires:

```bash
DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 scripts/check-desktop-platform-package-build.sh
```

on the target platform host.

## Verification

Run:

```bash
cargo check --locked -p deve_desktop --no-default-features
cargo test --locked -p deve_desktop --no-default-features -- --nocapture
cargo check --locked -p deve_desktop --features native-packaging
cargo test --locked -p deve_desktop --features native-packaging tauri -- --nocapture
scripts/check-native-track-boundary.sh
scripts/check-desktop-platform-package-build.sh
scripts/plan-coverage.sh --summary-missing-plan-ref
```

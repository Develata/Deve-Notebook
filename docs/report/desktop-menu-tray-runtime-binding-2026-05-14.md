# Desktop Menu/Tray Runtime Binding - 2026-05-14

本报告记录 `NPG-2c Desktop Menu/Tray Runtime Binding`。`docs/plan/` 仍是唯一权威；本批只在 Desktop `native-packaging` scope 内接入 Tauri menu/tray builder，不打开 native process adapter。

## Scope

- Plan basis: `08_ui_design_02_desktop.md#desktop-packaging-scaffold`, `14_tech_stack.md#native-packaging-dependency-gate`.
- Code scope: `apps/desktop` menu/tray binding, packaging metadata/tests, native guard scripts.
- Non-goal: run Tauri app runtime, spawn backend process, supervise child process, write ledger/vault/source-control/search/Git authority, claim platform release readiness.

## Result

- `native-packaging` now enables Tauri `tray-icon` API only inside `apps/desktop`.
- Desktop menu builder binds app/window/help menu items to UI intent ids.
- Desktop tray builder binds tray menu items to UI intent ids.
- Menu/tray event ids resolve to `DesktopMenuAction` / `DesktopTrayAction`; unknown ids fail closed to `None`.
- Guard scripts allow Tauri runtime imports only in `apps/desktop/src/menu_tray.rs`; other app/core paths remain blocked.
- Child-process runtime and native authority writes remain closed.

## Verification

- `cargo test -p deve_desktop --features native-packaging menu_tray -- --nocapture`
- `cargo test -p deve_desktop --features native-packaging packaging -- --nocapture`
- `cargo test -p deve_desktop --no-default-features -- --nocapture`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-packaging-gate.sh`
- `cargo check --workspace --locked --no-default-features`
- `cargo check -p deve_desktop --features native-packaging`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`

## Residual Gates

- Desktop platform package build/signing must be validated on target platform hosts before any release-ready claim.
- Native process adapter remains a separate gate.
- Mobile packaging dependency spike remains closed until explicitly selected.

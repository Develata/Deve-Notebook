# Desktop Menu/Tray Surface Declaration - 2026-05-14

本报告记录 `NPG-2b Desktop Menu/Tray Surface Declaration`。`docs/plan/` 仍是唯一权威；本批只声明 menu/tray action surface，不引入真实 Tauri menu/tray builder，不打开 native process adapter。

## Scope

- Plan basis: `08_ui_design_02_desktop.md#desktop-packaging-scaffold`, `14_tech_stack.md#native-packaging-dependency-gate`.
- Code scope: `apps/desktop` packaging metadata/tests and native guard scripts.
- Non-goal: import `tauri::menu` / `tauri::tray`, run Tauri runtime, spawn backend process, supervise child process, write ledger/vault/source-control/search/Git authority, claim platform release readiness.

## Result

- Declared Desktop menu ids and tray id under the `native-packaging` acceptance surface.
- Declared menu actions: show main window, open command palette, open settings, quit requested.
- Declared tray actions: show main window, toggle window visibility, quit requested.
- All actions remain UI intents only; no action opens process runtime or business authority writes.
- Tauri menu/tray runtime imports remain deferred.

## Verification

- `cargo test -p deve_desktop --features native-packaging packaging -- --nocapture`
- `cargo test -p deve_desktop --no-default-features -- --nocapture`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-packaging-gate.sh`
- `cargo check --workspace --locked --no-default-features`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`

## Residual Gates

- Real Tauri menu/tray builder binding remains a separate decision.
- Desktop platform package build/signing must be validated on target platform hosts before any release-ready claim.
- Native process adapter remains a separate gate.

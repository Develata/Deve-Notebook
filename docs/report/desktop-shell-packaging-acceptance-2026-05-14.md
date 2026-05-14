# Desktop Shell Manifest Acceptance - 2026-05-14

本报告记录 `NPG-2a Desktop Shell Manifest Acceptance`。`docs/plan/` 仍是唯一权威；本批只补 Desktop shell manifest acceptance，不打开 menu/tray runtime，不打开 Mobile dependency spike，不打开 native process adapter。

## Scope

- Plan basis: `08_ui_design_02_desktop.md#desktop-packaging-scaffold`, `14_tech_stack.md#native-packaging-dependency-gate`.
- Code scope: `apps/desktop` packaging metadata/tests and native guard scripts.
- Non-goal: run Tauri runtime, declare menu/tray runtime, build signed platform packages, spawn backend process, supervise child process, write ledger/vault/source-control/search/Git authority, claim macOS/Windows release readiness.

## Result

- Added Desktop `tauri.conf.json` shell manifest metadata for product name, identifier, main window, bundle metadata, and disabled updater artifacts.
- Extended `DesktopPackagingAcceptance` with shell acceptance metadata for installer metadata, updater-disabled state, session handoff gate, deferred menu/tray runtime, process runtime closure, and release-readiness non-claim.
- Guard scripts now verify the Desktop manifest and shell acceptance markers.
- Mobile remains dependency-deferred and no-Tauri.
- `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY` remains `DeferredUntilPackagingGate`.

## Verification

- `cargo test -p deve_desktop --features native-packaging packaging -- --nocapture`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-packaging-gate.sh`
- `cargo check --workspace --locked --no-default-features`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`

## Residual Gates

- Menu/tray runtime declaration remains a separate Desktop shell acceptance step.
- Desktop platform package build/signing must be validated on target platform hosts before any release-ready claim.
- Mobile dependency spike remains blocked until explicitly selected.
- Native process adapter remains a separate gate; shell packaging acceptance does not imply service supervision readiness.

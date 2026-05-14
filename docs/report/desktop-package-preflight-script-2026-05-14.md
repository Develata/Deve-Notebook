# Desktop Package Preflight Script - 2026-05-14

本报告记录 `NPG-2d Desktop Package Build Preflight Script`。`docs/plan/` 仍是唯一权威；本批只增加平台打包前的自动化预检，不执行 Tauri package build/signing。

## Scope

- Plan basis: `08_ui_design_02_desktop.md#desktop-packaging-scaffold`, `14_tech_stack.md#native-packaging-dependency-gate`.
- Code scope: Desktop preflight script, native packaging gate, release acceptance binding.
- Non-goal: run Tauri app runtime, build platform installer, sign artifacts, spawn backend process, write ledger/vault/source-control/search/Git authority, claim platform release readiness.

## Result

- Added `scripts/check-desktop-package-preflight.sh`.
- The script verifies default Desktop dependency tree remains no-Tauri.
- The script verifies `native-packaging` includes `tauri`, `tauri-build`, and `tray-icon`.
- The script runs Desktop default/no-packaging check and tests.
- The script runs Desktop `native-packaging` check plus menu/tray and packaging tests.
- `scripts/check-native-packaging-gate.sh` now also guards the `tray-icon` lock/tree surface.
- `REL-005` release acceptance now includes the desktop package preflight script.
- `docs/dev-runbook.md` guard-script list now includes the desktop package preflight script.

## Verification

- `scripts/check-desktop-package-preflight.sh`
- `scripts/check-native-packaging-gate.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `cargo fmt --check`
- `git diff --check`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`

## Residual Gates

- Desktop package build/signing must still run on target platform hosts before any release-ready claim.
- Native process adapter remains a separate gate.
- Mobile packaging dependency spike remains closed until explicitly selected.

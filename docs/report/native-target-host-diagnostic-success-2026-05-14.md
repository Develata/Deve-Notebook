# Native Target-host Diagnostic Success

Date: 2026-05-14

## Scope

This report records the first successful GitHub-hosted diagnostic pass for the
native target-host workflow after updating GitHub Actions runtime versions and
closing the Windows desktop icon scaffold gap.

No `docs/plan/` contract changed in this batch.

## Workflow

- Workflow: `Native Target Host`
- Run: https://github.com/Develata/Deve-Notebook/actions/runs/25865917415
- Head: `e0d4504a73a3c947d4f3a25ce16a0daeb6a453a3`
- Mode: `target=all`, `required_preflight=false`, `run_desktop_package_build=false`
- Result: success

## Target Jobs

- Desktop macOS: success
- Desktop Windows: success
- Mobile iOS: success

## Runtime Baseline

- `actions/checkout`: `v6`
- `actions/setup-node`: `v6`
- `actions/upload-artifact`: `v7`
- Node observed in evidence: `v24.15.0`
- Rust observed in evidence: `rustc 1.92.0`

## Evidence

- `target/native-target-host-evidence-download/deve-native-target-host-evidence-macos/desktop-macos.md`
- `target/native-target-host-evidence-download/deve-native-target-host-evidence-windows/desktop-windows.md`
- `target/native-target-host-evidence-download/deve-native-target-host-evidence-ios/mobile-ios.md`

All downloaded evidence files passed `scripts/check-native-target-host-evidence.sh`.

## Boundaries

- Desktop package build remained skipped by workflow input.
- iOS package execution remained closed.
- Process runtime gate remained closed.
- Native authority writes remained closed.

## Follow-Up

- The next target-host step is explicit desktop package execution with
  `run_desktop_package_build=true`.
- iOS package execution still requires a separate package execution gate before
  `cargo tauri ios init` or `cargo tauri ios build` is allowed.

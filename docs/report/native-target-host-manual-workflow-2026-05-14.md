# Native Target-host Manual Workflow

Date: 2026-05-14

## Scope

Added a manual GitHub Actions workflow for target-host diagnostics:

- `.github/workflows/native-target-host.yml`

This workflow is optional delivery evidence. It does not change
`.github/workflows/release.yml`, does not become the tag-triggered release
baseline, and does not modify `docs/plan/`.

## Behavior

- `target=desktop-macos` runs Desktop macOS target-host preflight on
  `macos-latest`.
- `target=desktop-windows` runs Desktop Windows target-host preflight on
  `windows-latest`.
- `target=mobile-ios` runs Mobile iOS preflight on `macos-latest`.
- `target=all` runs all three jobs.
- `required_preflight=false` keeps the workflow diagnostic by default.
- `required_preflight=true` makes target-host prerequisites fail closed.
- `run_desktop_package_build=true` runs Desktop package build after Desktop
  preflight.

## Boundary

- The workflow is `workflow_dispatch` only.
- The tag-triggered `release.yml` remains the only required release workflow.
- iOS package execution remains closed; the workflow does not run
  `cargo tauri ios init` or `cargo tauri ios build`.
- Process runtime remains closed.
- Native authority writes remain closed.

## Verification

- `scripts/check-release-baseline.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `cargo fmt --check`
- `git diff --check`

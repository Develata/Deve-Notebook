# Desktop Platform Package Build Decision

Date: 2026-05-14

## Scope

- Active queue item: Desktop platform package build decision.
- Source of truth: `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/14_tech_stack.md`, and current release acceptance docs.
- This batch does not modify `docs/plan/`.

## Decision

- Platform package build is not claimed on the current host.
- The current Desktop crate has native-packaging dependency and menu/tray preflight coverage, but it does not yet contain the real Tauri binary entrypoint or build script required for `cargo tauri build`.
- The current host also lacks the `cargo-tauri` CLI.
- A Linux/WSL package result would only validate that host target and must not be described as macOS or Windows release readiness.

## Added Gate

- Added `scripts/check-desktop-platform-package-build.sh`.
- Default mode runs the existing Desktop package preflight, reports target-host package prerequisites, and skips the actual package build when prerequisites are missing.
- `DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1` turns missing prerequisites into a hard failure and runs `cargo tauri build` when all prerequisites exist.

## Current Missing Prerequisites

- `apps/desktop/src/main.rs`
- `apps/desktop/build.rs`
- `cargo tauri` CLI

`apps/web/dist/index.html` is present in the current workspace.

## Boundary

- No Desktop runtime entrypoint was added.
- No child-process adapter was opened.
- No ledger, vault, source-control, search, Git, or `.notegit` authority moved into Desktop native code.
- No macOS, Windows, or signed installer readiness is claimed.

## Verification

Run:

```bash
scripts/check-desktop-platform-package-build.sh
scripts/check-release-baseline.sh
scripts/check-dev-runbook-baseline.sh
scripts/plan-coverage.sh --summary-missing-plan-ref
```

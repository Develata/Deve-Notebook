# Web Entry DOM No-Panic

Date: 2026-05-14

## Scope

- `apps/web/src/main.rs`
- `scripts/check-release-baseline.sh`

## Contract

- `docs/plan/08_ui_design_01_web.md#single-binary-distribution`
- `docs/plan/05_network.md#web-ws-runtime`

## Change

- Replaced direct `web_sys::window().unwrap()` and `window.document().unwrap()` in the WASM entry point with explicit `Option` checks.
- Missing browser `window` or `document` now logs an error and skips app mounting instead of panicking.
- Normal browser behavior remains unchanged: boot panel update, loading overlay hide, Leptos mount, and boot panel hide still run when DOM prerequisites exist.
- Added a release baseline guard to prevent reintroducing entry-point DOM unwraps.

## Verification

- `cargo check --locked -p deve_web --target wasm32-unknown-unknown`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`

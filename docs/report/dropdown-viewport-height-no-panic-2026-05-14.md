# Dropdown Viewport Height No-Panic - 2026-05-14

## Scope

- Runtime surface: Web dropdown placement helper.
- Plan basis: `docs/plan/08_ui_design_01_web.md#web-layout-persistence`.

## Change

- Replaced `window.expect("window")` in dropdown viewport measurement with an explicit `Option` chain and `0.0` fallback.
- Kept normal browser placement behavior unchanged when `window.innerHeight` is available.
- Added a native unit test for no-window fallback.
- Added UI baseline guards so dropdown placement cannot regain the window panic.

## Verification

- `cargo test -p deve_web dropdown -- --nocapture`
- `bash scripts/check-ui-z-index-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`

## Result

Dropdown placement remains bounded by viewport height in normal browser runtime and fails soft instead of panicking when no browser window is available.

# Web Activity Bar More Contract

Date: 2026-05-17

## Scope

- Plan anchor: `docs/plan/08_ui_design_01_web.md#web-layout-persistence`.
- Contract closed: Activity Bar `More...` row click selects a view; `Pin/Unpin` is a separate operation.
- No `docs/plan/` change.

## Changes

- Added stable Activity Bar More DOM markers:
  - `data-deve-activity-more-button`
  - `data-deve-activity-more-item`
  - `data-deve-activity-more-pin-action`
- Extracted pure menu-state helpers:
  - `activity_more_after_item_click()`
  - `activity_more_after_pin_click(open)`
  - `toggle_activity_more_pin(...)`
- Bound `UI-WEB-004` to `scripts/check-ui-desktop-baseline.sh`.

## Verification

- `cargo fmt --check`
- `cargo test -p deve_web activity_more -- --nocapture`
- `cargo clippy -p deve_web --all-targets -- -D warnings`
- `bash scripts/check-ui-desktop-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-architecture-registry.sh`
- `bash scripts/plan-coverage.sh`

## Result

- Activity Bar More behavior is now regression-bound without changing UI authority or opening native/desktop/mobile runtime gates.
- Acceptance counters: automated `147`, feature walkthrough `54`, manual `0`, unbound `0`.
- Plan coverage: blocking `0`, soft warnings `18`.

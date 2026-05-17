# Web Repo Switcher Contract

Date: 2026-05-17

## Scope

- Plan anchor: `docs/plan/08_ui_design_01_web.md#web-layout-persistence`.
- Contract closed: Repo Switcher trigger and menu items use button semantics; outside click dismisses the menu.
- No `docs/plan/` change.

## Changes

- Added stable Repo Switcher DOM markers:
  - `data-deve-repo-switcher-trigger`
  - `data-deve-repo-switcher-menu`
  - `data-deve-repo-switcher-backdrop`
  - `data-deve-repo-switcher-item`
- Added pure state helpers:
  - `repo_switcher_after_trigger_click(open)`
  - `repo_switcher_after_outside_click()`
  - `repo_switcher_after_item_click()`
- Bound `UI-WEB-005` to `scripts/check-ui-desktop-baseline.sh`.

## Verification

- `cargo fmt --check`
- `cargo test -p deve_web repo_switcher -- --nocapture`
- `cargo clippy -p deve_web --all-targets -- -D warnings`
- `bash scripts/check-ui-desktop-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/plan-coverage.sh`

## Result

- Repo Switcher behavior is now regression-bound without changing repo-switch authority or opening native/desktop/mobile runtime gates.
- Acceptance counters: automated `148`, feature walkthrough `54`, manual `0`, unbound `0`.

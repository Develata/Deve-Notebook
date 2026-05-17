# Web Activity Bar More Browser Smoke

Date: 2026-05-17

## Scope

- Contract: `UI-WEB-004`.
- Plan anchor: `docs/plan/08_ui_design_01_web.md#web-layout-persistence`.
- Runtime: embedded Web frontend served by `deve_cli serve --dev`.
- No `docs/plan/` change.

## Environment

- Web build: `NO_COLOR=true BROWSERSLIST_IGNORE_OLD_DATA=true trunk build`
- Data root: `/tmp/deve-activity-more-smoke-20260517-105914`
- Init:
  - `DEVE_LEDGER_DIR=/tmp/deve-activity-more-smoke-20260517-105914/ledger`
  - `DEVE_VAULT_PATH=/tmp/deve-activity-more-smoke-20260517-105914/vault`
  - `cargo run -p deve_cli --bin deve_cli -- init --path /tmp/deve-activity-more-smoke-20260517-105914`
- Server:
  - `DEVE_STATIC_DIR=apps/web/dist`
  - `cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3134`
- Browser:
  - `chrome-mcp http://127.0.0.1:3134/`

## Checks

- Web shell reached `Ready`.
- `data-deve-activity-more-button="activity-more-button"` existed before opening the menu.
- Clicking `更多操作` exposed all four row item markers:
  - `activity_more_item_explorer`
  - `activity_more_item_search`
  - `activity_more_item_source_control`
  - `activity_more_item_extensions`
- Clicking row item `资源管理器` closed the More menu.
- Reopening `更多操作` and clicking Search `Pin/Unpin` changed the pin action from `取消固定` to `固定`.
- After Search `Pin/Unpin`, `menuStillOpen=true`; all four menu item markers and all four pin action markers remained visible.

## Result

- Browser smoke result: pass.
- Row click and Pin/Unpin are separate in the actual DOM event path.
- No `UnsupportedVersion`, disconnected lockout, stale-scope lockout, auth lockout, or application panic was observed during this focused smoke.

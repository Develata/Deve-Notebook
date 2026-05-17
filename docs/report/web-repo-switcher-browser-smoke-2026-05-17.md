# Web Repo Switcher Browser Smoke

Date: 2026-05-17

## Scope

- Contract: `UI-WEB-005`.
- Plan anchor: `docs/plan/08_ui_design_01_web.md#web-layout-persistence`.
- Runtime: embedded Web frontend served by `deve_cli serve --dev`.
- No `docs/plan/` change.

## Environment

- Web build: `NO_COLOR=true BROWSERSLIST_IGNORE_OLD_DATA=true trunk build`
- Data root: `/tmp/deve-repo-switcher-smoke-20260517-111640`
- Init:
  - `DEVE_LEDGER_DIR=/tmp/deve-repo-switcher-smoke-20260517-111640/ledger`
  - `DEVE_VAULT_PATH=/tmp/deve-repo-switcher-smoke-20260517-111640/vault`
  - `cargo run -p deve_cli --bin deve_cli -- init --path /tmp/deve-repo-switcher-smoke-20260517-111640`
- Server:
  - `DEVE_STATIC_DIR=apps/web/dist`
  - `cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3135`
- Browser:
  - `chrome-mcp http://127.0.0.1:3135/`

## Checks

- Web shell reached `Ready`.
- `data-deve-repo-switcher-trigger="repo-switcher-trigger"` existed before opening the menu.
- Trigger element was `BUTTON` with `type="button"`.
- Clicking the trigger exposed `data-deve-repo-switcher-menu="visible"` with `role="menu"`.
- Menu exposed one repo item for `default` with `data-deve-repo-switcher-item="repo-switcher-item"`, `type="button"`, and `role="menuitem"`.
- Clicking `data-deve-repo-switcher-backdrop="repo-switcher-outside"` closed the menu.
- Reopening the menu and clicking the repo item closed the menu after the repo switch request completed.
- `/api/node/role` returned `200`.
- `/api/repo/docs` returned `200`.
- Browser console `error` / `warn` count after the final reload was `0`.

## Runtime Fix

- The first smoke pass exposed Leptos warnings from modal focus restore setup.
- Root cause: `focus_scope::attach_modal_focus_restore_effect` called its `is_open` callback before creating the tracking effect.
- Fix: initialize the previous open state as closed and read `is_open` only inside the effect.

## Result

- Browser smoke result: pass.
- Repo Switcher trigger/menu/item/outside-click behavior matches the Web shell contract in the actual DOM event path.
- No `UnsupportedVersion`, disconnected lockout, stale-scope lockout, auth lockout, console warning, console error, or application panic was observed after the fix.

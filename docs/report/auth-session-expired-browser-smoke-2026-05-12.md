# Auth Session Expired Browser Smoke - 2026-05-12

## Scope

- Acceptance: `AUTH-011`, `NET-013`
- Plan anchors: `09_auth#unauthorized-disconnected-ui`, `09_auth#session-probe-policy`
- Runtime: Chrome MCP, isolated dev server, static frontend from `apps/web/dist`

## Environment

- Data root: `/tmp/deve-auth-session-smoke-WEh28S`
- Backend:
  `DEVE_LEDGER_DIR=/tmp/deve-auth-session-smoke-WEh28S/ledger DEVE_VAULT_PATH=/tmp/deve-auth-session-smoke-WEh28S/vault DEVE_STATIC_DIR=/home/develata/gitclone/Deve-Notebook/apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3012`
- Frontend dist rebuilt before the final run:
  `NO_COLOR=true BROWSERSLIST_IGNORE_OLD_DATA=true trunk build`
- Loaded assets:
  `deve_web-63aefcf470ac76cf.js`,
  `deve_web-63aefcf470ac76cf_bg.wasm`

## Browser Evidence

- Initial workspace loaded as authenticated dev session.
- `/api/auth/status` returned `{ "authenticated": true }`.
- External session invalidation:
  `fetch('/api/auth/logout', { method: 'POST', credentials: 'include' })`
  returned HTTP `200`.
- Immediate `/api/auth/status` returned `{ "authenticated": false }`.
- After the periodic auth probe, UI switched to the login page.
- Login page visible: `Deve 笔记 / 登录以继续 / 用户名 / 密码 / 登录`.
- `Reconnecting...` was not visible.
- `[data-deve-disconnect-overlay="lockdown"]` was absent.
- `Boot Error` / `Global Error` was absent.
- Browser console had no `error` or `warn` messages.
- Explicit UI logout button path was also verified after re-login:
  `退出登录` returned to the login page with no disconnected overlay and no console errors.

## Bug Fixed During Smoke

The first browser run exposed a real lifecycle bug:

- When session expiry unmounted `MainLayout`, the old `WsService` connection manager and related async tasks could keep touching Leptos arena signals after their owner was disposed.
- Browser symptom: repeated `Tried to access a reactive value that has already been disposed` panics and `Boot Error` overlays.
- Server symptom: reconnect storm and rate-limit noise from stale WS reconnect attempts.

Fix:

- Added `ConnectionLifecycle` for `WsService`.
- Registered owner cleanup to mark the connection lifecycle inactive.
- Made WS manager, node-role probe and connected session stop after lifecycle shutdown.
- Replaced post-cleanup signal writes in this path with lifecycle-gated `try_set` / `try_update`.
- Updated auth guard scripts to bind to lifecycle-aware unauthorized/disconnected status writes.

## Verification

- `trunk build`: passed
- `cargo fmt --check`: passed
- `cargo test -p deve_web connection -- --nocapture`: `15 passed`
- `cargo test -p deve_web status_summary -- --nocapture`: `11 passed`
- `cargo test -p deve_web auth_probe -- --nocapture`: `4 passed`
- `cargo test -p deve_web incoming -- --nocapture`: `11 passed`
- `scripts/check-auth-baseline.sh`: passed
- `scripts/check-auth-unauthorized-state.sh`: passed
- `scripts/check-acceptance-bindings.sh`: passed
- `scripts/plan-coverage.sh`: blocking `0`, soft warnings `72`

## Result

`AUTH-011` and `NET-013` are closed for this browser smoke. Session-expired and normal disconnected/reconnecting UI states remain separated, and stale WS tasks no longer write disposed runtime signals after auth teardown.

# Web PWA Manifest Browser Smoke

Date: 2026-05-17

## Scope

- Contract: `UI-WEB-006`.
- Plan chapter: `docs/plan/08_ui_design_01_web.md` §5 PWA Support.
- Runtime: embedded Web frontend served by `deve_cli serve --dev`.
- No `docs/plan/` change.
- No service worker, offline authority, native runtime, or browser storage behavior was introduced.

## Environment

- Data root: `/tmp/deve-pwa-manifest-smoke-20260517-113353`
- Init:
  - `DEVE_LEDGER_DIR=/tmp/deve-pwa-manifest-smoke-20260517-113353/ledger`
  - `DEVE_VAULT_PATH=/tmp/deve-pwa-manifest-smoke-20260517-113353/vault`
  - `cargo run -p deve_cli --bin deve_cli -- init --path /tmp/deve-pwa-manifest-smoke-20260517-113353`
- Server:
  - `DEVE_STATIC_DIR=apps/web/dist`
  - `cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3136`
- Browser:
  - `chrome-mcp http://127.0.0.1:3136/`

## Checks

- Root page reached `Ready`.
- Root page contained `<link rel="manifest" href="/manifest.json" />`.
- Root page contained `<meta name="theme-color" content="#1e1e1e" />`.
- Direct local HTTP fetch of `/manifest.json` returned `200`.
- `/manifest.json` response header included `content-type: application/json`.
- Browser fetch of `/manifest.json` returned:
  - `display = "standalone"`
  - `theme_color = "#1e1e1e"`
  - `start_url = "/"`
  - `scope = "/"`
- Browser console `error` / `warn` count was `0`.

## Result

- Browser smoke result: pass.
- PWA manifest metadata is available through the actual static-file serving path.
- No `UnsupportedVersion`, disconnected lockout, auth lockout, console warning, console error, or application panic was observed.

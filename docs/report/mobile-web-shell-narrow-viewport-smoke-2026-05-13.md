# Mobile Web Shell Narrow-Viewport Smoke - 2026-05-13

本报告记录 `Mobile Web shell narrow-viewport smoke`。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Scope

- `docs/plan/08_ui_design_03_mobile.md`
- `docs/features/08_ui_design_03_mobile.md`
- `docs/acceptance-cases/05_ui.md` UI-MOB-001 / UI-MOB-002 / UI-MOB-005 / UI-MOB-007 / UI-MOB-008 / UI-MOB-009 / UI-MOB-011 / UI-MOB-012
- `docs/acceptance-cases/13_ui_mobile_chat_regression.md`

## Environment

- Frontend build: existing `apps/web/dist` from the current release build output.
- Backend: `DEVE_LEDGER_DIR=/tmp/deve-mobile-shell-20260513-9svdNe/ledger DEVE_VAULT_PATH=/tmp/deve-mobile-shell-20260513-9svdNe/vault DEVE_STATIC_DIR=apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 31996`
- Browser URL: `http://127.0.0.1:31996/`
- Data root: `/tmp/deve-mobile-shell-20260513-9svdNe`
- Chrome MCP viewport emulation: `375x812`, `deviceScaleFactor=2`, `isMobile=true`, `hasTouch=true`
- Auth: development defaults, login `admin / admin`

## Results

Passed:

- Real emulated viewport reported `innerWidth=375`, `innerHeight=812`, `visualViewport.scale=1`.
- Root shell exposed `data-deve-layout-mode="mobile"` and occupied `375x812`.
- Desktop resize handles were absent: `.resizer-handle` count was `0`.
- Mobile top-bar touch targets were present and minimum measured touch target was `44px`.
- Left drawer opened from the top-bar file-tree button and exposed `data-deve-mobile-drawer="left"` with `data-deve-mobile-drawer-open="true"`.
- Left drawer tab strip exposed `data-deve-mobile-sidebar-icon-tabs="visible"`.
- Search entry from the mobile drawer closed the drawer and opened a top sheet with `data-deve-search-sheet-position="top"`, `data-deve-search-sheet-handle="top"`, and isolated search results scrolling.
- Escape closed the search sheet without leaving the mobile shell.
- Creating/opening `Untitled.md` exposed the editor surface and mobile outline toggle.
- Outline toggle opened the right drawer with `data-deve-mobile-drawer="right"` and `data-deve-mobile-drawer-open="true"`.
- Bottom bar defaulted to `collapsed / single`, expanded to `expanded` with details, and collapsed after outside click.
- AI Chat chip opened a fullscreen mobile chat page with `data-deve-mobile-chat-page="fullscreen"` and `data-deve-mobile-chat-fullscreen="true"`.
- Mobile chat input and send button markers were present; send target measured `44x44`.
- Bottom bar was hidden while mobile chat was fullscreen and restored after closing chat.
- Browser console contained only expected runtime logs; no `error`, `warn`, `UnsupportedVersion`, stale-scope lockout, auth lockout, or uncaught application panic was observed.
- Network requests observed during the selected page run returned HTTP `200`; no failed resource, auth, node-role, WASM, or editor-bundle request was observed.

## Notes

- The smoke used Chrome MCP viewport emulation after login in an isolated browser context, not a desktop-width JS-only breakpoint override.
- No code changes were needed in this batch.

## Verification

已运行：

- `bash scripts/check-mobile-baseline.sh`
- `cargo test -p deve_web mobile_ -- --nocapture`
- Chrome MCP browser smoke as described above

结果：

- Mobile baseline guard: pass
- Mobile unit/regression tests: pass, 78 tests
- Browser narrow-viewport mobile shell smoke: pass

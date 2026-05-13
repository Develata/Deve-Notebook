# Mobile Residual Interaction Smoke

Date: 2026-05-13

## Scope

- Plan source: `docs/plan/08_ui_design_03_mobile.md`.
- Acceptance binding: `docs/acceptance-cases/05_ui.md` `UI-MOB-004`, `UI-MOB-006`, `UI-MOB-013`, `UI-MOB-014`, `UI-MOB-017`, `UI-MOB-018`; `docs/acceptance-cases/13_ui_mobile_chat_regression.md`.
- Runtime surface: Web mobile shell at `375x812`, mobile accessory toolbar, top search sheet result scrolling, mobile AI Chat error/readability path, mobile Source Control diff open/close.
- Data root: isolated temp root `/tmp/deve-mobile-residual-20260513-Qe1OQ8`.
- Server: `DEVE_LEDGER_DIR=/tmp/deve-mobile-residual-20260513-Qe1OQ8/ledger DEVE_VAULT_PATH=/tmp/deve-mobile-residual-20260513-Qe1OQ8/vault DEVE_STATIC_DIR=apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 32120`.

## Automated Guards

- `bash scripts/check-mobile-baseline.sh` -> passed
- `cargo test -p deve_web mobile_ -- --nocapture` -> passed, 78 tests
- `cargo test -p deve_cli plugin_error_result -- --nocapture` -> passed
- `cargo test -p deve_cli plugin_result_ -- --nocapture` -> passed
- `cargo test -p deve_core test_chat_without_api_key_returns_error -- --nocapture` -> passed
- `cargo test -p deve_web plugin_response_matches -- --nocapture` -> passed
- `cargo test -p deve_web unrelated_plugin_response_is_ignored -- --nocapture` -> passed
- `cargo test -p deve_web mobile_chat_error -- --nocapture` -> passed
- `bash scripts/smoke-web-release-build.sh` -> passed

## Browser Smoke

Chrome MCP viewport:

- `width=375`
- `height=812`
- `deviceScaleFactor=2`
- `isMobile=true`
- `hasTouch=true`

Observed:

- Layout marker was `data-deve-layout-mode="mobile"`.
- Mobile accessory toolbar became visible when `visualViewport.height` was reduced to `500`, with `data-deve-keyboard-offset="312"` and buttons `⇥ H • ☑ B I <> ↩`.
- Without the simulated keyboard, toolbar stayed hidden as expected in desktop Chrome mobile emulation.
- Search top sheet exposed `data-deve-search-sheet-position="top"` and `data-deve-search-results-scroll="isolated"`.
- Scrolling and swipe-like events inside search results changed result scroll position and did not close the sheet.
- Mobile AI Chat opened as fullscreen and hid the bottom bar.
- Long/error chat messages carried mobile readability markers: `data-deve-mobile-chat-wrap="break-words"`, `data-deve-mobile-chat-code-scroll="horizontal"`, `data-deve-mobile-chat-timestamp="visible"`.
- Missing AI API key produced a structured chat error banner, `data-deve-chat-error-banner="visible"`.
- Retry button was visible and clicking it started a new request using the last prompt.
- Mobile Source Control diff opened as `.diff-view-mobile` with unified diff content.
- While diff was open and the drawer was closed, AI Chat chip and mobile accessory toolbar were both absent.
- Diff close button measured `44x44` and returned to the editor.

Expected console warnings:

- AI plugin requests without API key logged warning messages containing `No AI API key configured`.
- No unrelated console `error`, `UnsupportedVersion`, auth lockout, stale-scope lockout, or application panic was observed.

## Defects Fixed

- Bundled `ai-chat` returned missing-provider failures as `type: text` success payloads. It now returns `type: error`.
- CLI plugin handler now converts plugin `type: error` payloads into structured `PluginResponse.error(RequestFailed)`.
- Chat panel error effect now matches plugin responses by local pending request or by existing chat placeholder `req_id`, and records handled responses to avoid replaying stale plugin errors.

## Result

Mobile residual interactions are browser-smoke verified for keyboard toolbar gating, search result scroll isolation, AI Chat readability/error/retry behavior, and mobile diff open/close visibility rules.

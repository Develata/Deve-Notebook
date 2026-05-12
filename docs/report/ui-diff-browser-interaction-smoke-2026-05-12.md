# UI Diff Browser Interaction Smoke - 2026-05-12

本报告记录 `UI-DIFF-*` 剩余浏览器交互点验。`docs/plan/` 仍是唯一权威；本文件只记录实机证据。

## Environment

- Data root: `/tmp/deve-ui-diff-smoke-cuuZAn`
- Backend: `DEVE_LEDGER_DIR=/tmp/deve-ui-diff-smoke-cuuZAn/ledger DEVE_VAULT_PATH=/tmp/deve-ui-diff-smoke-cuuZAn/vault DEVE_STATIC_DIR=/home/develata/gitclone/Deve-Notebook/apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001`
- Browser URL: `http://127.0.0.1:3001/`
- Frontend delivery: `static-dir-override`; no separate `trunk serve`
- Browser tool: Chrome MCP

## Fixture

- Cleared browser `localStorage` / `sessionStorage` before the run to remove stale editor ops from earlier sessions.
- Created `diff-smoke.md` through the Web UI.
- Wrote a baseline document through CodeMirror / WebWrite so Ledger projection remained the authority.
- Modified `/tmp/deve-ui-diff-smoke-cuuZAn/vault/default/diff-smoke.md` externally to create watcher-driven Source Control pending state.
- Source Control displayed `diff-smoke.md` as `M`.

## Desktop Diff Evidence

Passed:

- Clicking the Source Control change item opened `DiffView`.
- Header showed `+4`, `-3`, cache state, cache ratio, algorithm label and compute time.
- Hunk button navigation changed index `1/7 -> 2/7 -> 1/7`.
- Keyboard navigation changed index through `]` and `F7`, ending at `3/7`.
- Fold row was visible as `... 19 行未变更（点击展开）`; clicking it expanded folded content and removed fold buttons.
- Context selector exposed values `3 / 5 / 8`; selecting `8` updated the control value.
- Word-level replace highlights rendered changed words as separate DOM text nodes.
- Console error/warn list after the run was empty.

## Mobile Diff Evidence

Passed:

- Reopened the page with `innerWidth = 390` injected before app boot.
- App entered the mobile runtime branch and rendered `.diff-view-mobile`.
- Mobile Source Control opened the same pending diff.
- Mobile edit mode rendered `textarea[name='diff-edit-mobile']`.
- Debounce behavior matched the 150ms contract:
  - before input: `+5`, `1/8`, no computing indicator
  - 80ms after input: still `+5`, `1/8`, computing indicator visible
  - 170ms after input: updated to `+6`, `1/9`, computing indicator cleared
  - 290ms after input: remained stable at `+6`, `1/9`

Constraint:

- Chrome MCP in this environment did not expose a viewport resize tool, and Windows Chrome window lookup did not find a titled Deve-Note window. The mobile pass therefore validates the app runtime breakpoint branch, not a physical browser window resize.

## Automated Verification

已运行：

- `cargo test -p deve_web diff_ -- --nocapture`
- `scripts/check-source-control-baseline.sh`

结果：

- `deve_web diff_`: `73 passed`
- `source-control-baseline-check: ok`

## Result

`UI Diff browser interaction smoke` 可关闭。剩余的物理移动窗口尺寸 smoke 不是 blocking gap；若后续 Chrome MCP 增加 viewport resize 能力，可作为增强验收补跑。

# Search Disabled Fail-Closed Smoke - 2026-05-12

本报告记录 `SEARCH-002` 浏览器 fail-closed 点验。`docs/plan/` 仍是唯一权威；本文件只记录实机证据。

## Environment

- Data root: `/tmp/deve-search-disabled-smoke-sBZGQN`
- Backend: `DEVE_LEDGER_DIR=/tmp/deve-search-disabled-smoke-sBZGQN/ledger DEVE_VAULT_PATH=/tmp/deve-search-disabled-smoke-sBZGQN/vault DEVE_STATIC_DIR=/home/develata/gitclone/Deve-Notebook/apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001`
- Search feature: not compiled
- Profile: `standard`
- Browser URL: `http://127.0.0.1:3001/`
- Browser tool: Chrome MCP

## Browser Evidence

Passed:

- Web shell reached `Ready`.
- Search surface opened from the sidebar.
- Query `?needle` triggered the search path while runtime search was unavailable.
- UI showed user-visible feedback: `搜索不可用: Search feature not enabled`.
- Search dialog stayed open with `未找到结果。`, and no stale result entry was visible.
- Bottom status remained `就绪`.
- Current-page console error/warn list was empty.

## Automated Verification

已运行：

- `scripts/check-search-baseline.sh`
- `cargo test -p deve_cli search -- --nocapture`
- `cargo test -p deve_cli browser_search_rejects_stale_scope_before_handler -- --nocapture`
- `cargo test -p deve_web message_protocol -- --nocapture`

结果：

- `search-baseline-check: ok`
- `deve_cli search`: `3 passed`
- `browser_search_rejects_stale_scope_before_handler`: `1 passed`
- `deve_web message_protocol`: `11 passed`

## Result

`Search disabled / low-spec fail-closed browser smoke` 可关闭。本轮覆盖未启用 `search` feature 的 disabled path；LowSpec path 由同一服务端 unavailable contract 与 config/profile tests 兜底，不再作为当前队列阻塞项。

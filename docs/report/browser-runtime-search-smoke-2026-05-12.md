# Browser Runtime And Search Smoke - 2026-05-12

本报告记录 Chrome MCP 实机 smoke。测试使用隔离数据根，不依赖 checked-in `ledger/` 或 `vault/`。

## Environment

- Backend: `cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001`
- Frontend: `NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080`
- Search rerun: `cargo run -p deve_cli --features search --bin deve_cli -- serve --dev --port 3001`
- Browser URL: `http://127.0.0.1:8080/`
- Data root: `/tmp/deve-browser-e2e-*`

## Runtime E2E Result

Passed:

- development login with `admin/admin`
- Web shell reached `Ready`
- created `e2e-note.md`
- edited Markdown content
- refreshed page and reconnected
- reopened the document after reload
- editor content persisted
- no current-page console error/warn after stable reload
- no observed `UnsupportedVersion`, `scope mismatch`, stale-scope lockout, or disconnected lockout

## Search Smoke Result

Initial finding:

- `?note` returned `e2e-note.md 全文匹配`
- selecting the result reset the query to `?` and kept the search dialog open
- this violated `docs/features/operations/search_query.md` expectation that selecting a result opens the document and closes Search surface

Fix:

- `apps/web/src/components/sidebar/mod.rs` now opens the search overlay only on transition from non-Search view to Search view.
- This prevents Sidebar remount or same-view refresh from reopening Search with mode `?` after a result selection.

Post-fix result:

- `?note` returned `e2e-note.md 全文匹配`
- pressing Enter selected the result
- Search dialog closed
- `e2e-note.md` remained open with persisted content
- bottom status remained `就绪`
- current-page console error/warn list was empty

## Verification

已运行：

- `cargo test -p deve_web search_overlay_opens_only_on_search_view_transition -- --nocapture`
- `cargo test -p deve_web mobile_sidebar -- --nocapture`
- `scripts/check-mobile-baseline.sh`
- `scripts/check-search-baseline.sh`


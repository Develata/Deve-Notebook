# Search Acceptance Cases

这些用例覆盖全文搜索的当前可验收能力。当前实现是 repo-scoped baseline scan；
Tantivy 增量索引仍是 future optimization，不作为本文件阻塞项。

- case_id: SEARCH-001
  goal: Standard + `search` feature 下全文搜索返回当前 repo scope 的结果。
  preconditions:
    - `deve_cli` 使用 `--features search` 构建或运行
    - 当前 profile 不是 `low-spec`
    - 当前 browser scope 已稳定绑定 `repo_id` 与 `scope_nonce`
    - 当前 repo 中存在正文包含 `needle` 的 Markdown 文档
    - 记录该文档的路径为 `expected.md`
    - Chrome MCP 手工 smoke 可按 `docs/features/operations/search_query.md` 使用 `?note`
      验证默认开发数据路径
  steps:
    - ui_open_search: true
    - ui_type: "?needle"
    - wait_for_ws: "SearchResults"
    - run: scripts/check-search-baseline.sh
    - run: cargo test -p deve_cli --features search search -- --nocapture
  assertions:
    - ws_assert: SearchResults.request_id_matches_pending true
    - ws_assert: SearchResults.repo_id_eq_current true
    - ws_assert: SearchResults.branch_eq_current true
    - ws_assert: SearchResults.scope_nonce_eq_current true
    - ws_assert: SearchResults.results_contain_doc_path "expected.md"
    - ui_assert: search_results_contain_doc_path "expected.md"
    - ui_assert: search_result_detail_contains "Full-text match"
  notes:
    - 当前 baseline payload 为 `(doc_id, path, score)`，UI 不承诺显示正文 snippet 或 query highlight。

- case_id: SEARCH-002
  goal: LowSpec 或未启用 `search` feature 时全文搜索 fail-closed 并显示用户可见反馈。
  preconditions:
    - `DEVE_PROFILE=low-spec` 或 `deve_cli` 未启用 `search` feature
    - 当前 browser scope 已稳定绑定
  steps:
    - ui_open_search: true
    - ui_type: "?needle"
    - wait_for_ws: "ProtocolError"
    - run: scripts/check-search-baseline.sh
    - run: cargo test -p deve_cli search -- --nocapture
    - run: cargo test -p deve_web message_protocol -- --nocapture
  assertions:
    - ws_assert: ProtocolError.code_eq "RequestFailed"
    - ws_assert: ProtocolError.scope_nonce_eq_current true
    - ui_assert: sync_banner_contains "Search unavailable"
    - ui_assert: search_pending_request_cleared true
    - ui_assert: stale_search_results_cleared true

- case_id: SEARCH-003
  goal: SearchResults stale scope / stale request / stale repo 必须被前端丢弃。
  preconditions:
    - 当前 browser scope 已稳定绑定
    - 已发出一次 Search 请求并记录 pending request id
  steps:
    - inject_ws: SearchResults with stale request_id
    - inject_ws: SearchResults with stale repo_id
    - inject_ws: SearchResults with stale branch
    - inject_ws: SearchResults with stale scope_nonce
    - run: scripts/check-search-baseline.sh
    - run: cargo test -p deve_web message_dispatch_gate -- --nocapture
  assertions:
    - ui_assert: search_results_unchanged true
    - ui_assert: current_doc_unchanged true

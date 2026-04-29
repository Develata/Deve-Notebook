# Search / Settings Boundary Audit - 2026-04-29

## Decision

P1 Search/settings current-boundary audit 已完成。本批次补两处硬边界：Search WebSocket route
对过期 browser `scope_nonce` 的专项测试，以及 core `Config` 对 `ui.*` 键的真实反序列化/打印支持，
避免 CLI 可写但 `config print` 不可见。

## Search Current Boundary

当前可验收 Search：

- 编译条件：`deve_cli --features search`。
- 运行条件：profile 不是 `low-spec`。
- 数据源：当前 repo scope 下的 ledger docs，handler 重建内容后做 baseline scan。
- Wire payload：`SearchResults { request_id, repo_id, branch, scope_nonce, results: Vec<(doc_id, path, score)> }`。
- Web gate：只接受 request id、repo、branch、scope nonce 同时匹配的结果。
- 不可用路径：未编译 `search` feature、`low-spec` profile、缺失/stale browser scope 都返回结构化 `ProtocolError`。

明确 future：

- Tantivy 常驻 index service。
- 持久 search index。
- snippet、query highlight、richer ranking。
- search index 参与 ledger/source-control authority。

本批次新增 `browser_search_rejects_stale_scope_before_handler`，保证 stale Search request 在 route guard 阶段被拒绝，不进入 handler。
同时新增 `scope_search_scans_remote_branch_documents`，确认 remote branch scope 会扫描
`RepoType::Remote(peer_id, repo_id)`，不会混入同 repo 本地文档。

文档同步项：

- `search_query.md` 已改为当前真实行为：输入 `?query` 后 100ms debounce 自动触发全文搜索，而不是 Enter-only submit。
- `repo_open_doc.md` 已把 Quick Open current entry 收口到 `UnifiedSearch` / `FileProvider`，避免把预留的 `quick_open/mod.rs` 当成真实入口。

## Settings Current Boundary

当前可验收 Settings/config：

- `config.toml` 是当前 runtime settings 文件。
- `deve config print` 输出有效 runtime config。
- `deve config set <key> <value>` 只写白名单键，并在写盘前验证仍可反序列化为 `deve_core::config::Config`。
- `ui.*` 已进入 core `UiConfig`，不再只是 TOML 中被保留但 effective config 不可见的附加表。
- `docs/plan/13_settings.md` 与 `apps/cli/src/commands/config.rs` 的支持键通过测试保持一致。
- Browser UI prefs 仍是本地 UI 偏好层，不承载 repo authority、session secret、peer private key 或业务事实。

明确 future：

- `/api/settings` server-backed Settings API。
- 独立 `settings.toml`。
- 统一 GUI 持久化所有 runtime settings。
- Settings 页面直接改写 authority state。

## Verification Targets

- `scripts/check-search-baseline.sh`
- `scripts/check-cli-settings-baseline.sh`
- `cargo test -p deve_cli browser_search_rejects_stale_scope_before_handler -- --nocapture`
- `cargo test -p deve_cli --features search scope_search_scans_remote_branch_documents -- --nocapture`
- `cargo test -p deve_cli config -- --nocapture`
- `cargo test -p deve_core config -- --nocapture`
- `cargo test -p deve_web message_dispatch_gate -- --nocapture`

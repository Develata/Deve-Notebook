# search_query.md - 全文搜索操作流

## Metadata

- `Flow ID`: `flow.search.query`
- `Domain`: `search`
- `Related Feature Chapters`: `docs/features/08_ui_design.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `SEARCH-001`, `SEARCH-002`, `SEARCH-003`, `UI-DESK-003`, `UI-MOB-007`

## Operations

### `op.search.open`

- `Name`: `Open Search Surface`
- `Surface`: `keyboard-shortcut-or-sidebar`
- `Trigger`: Search 入口、移动端 Search tab、或全局搜索快捷键
- `Preconditions`: 应用主界面已加载
- `Immediate Result`: Unified Search 或 Search 面板显示
- `Application Entry`: `apps/web/src/components/search_box/`, `apps/web/src/components/mobile_layout/drawers/`

### `op.search.type-query`

- `Name`: `Type Search Query`
- `Surface`: `search-input`
- `Trigger`: 在搜索输入框键入全文查询
- `Preconditions`: Search surface 已打开
- `Immediate Result`: 查询草稿更新
- `Application Entry`: `apps/web/src/components/search_box/mod.rs`

### `op.search.submit`

- `Name`: `Submit Search Query`
- `Surface`: `search-input`
- `Trigger`: Enter 或搜索提交动作
- `Preconditions`: workspace ready，repo scope 稳定，未处于 branch/repo switch
- `Immediate Result`: 发送 `ClientMessage::Search`
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks_misc.rs`

### `op.search.receive-results`

- `Name`: `Receive Search Results`
- `Surface`: `search-results`
- `Trigger`: 服务端返回 `ServerMessage::SearchResults`
- `Preconditions`: request id 与 `scope_nonce` 匹配当前 workspace
- `Immediate Result`: 搜索结果列表更新
- `Application Entry`: `apps/web/src/hooks/use_core/effects/message_dispatch_runtime.rs`

## Response Flow

1. 用户打开搜索入口，应用显示统一搜索 UI。
2. 用户输入查询，前端更新搜索草稿。
3. 提交时，前端检查 loading、branch/repo switch 和 `scope_nonce`。
4. 前端发送 `ClientMessage::Search { request_id, query, limit, scope_nonce }`。
5. CLI server 校验 browser scope nonce，并进入 search handler。
6. Standard profile + `search` feature 下进入 repo-scoped baseline search；当前实现按当前
   `repo_id/branch/scope_nonce` 扫描已登记文档内容，完整 Tantivy 增量索引仍是后续优化路径。
7. LowSpec、未启用 `search` feature、或当前 repo scope 无效时必须 fail closed 并返回结构化错误。
8. 前端只接受 request id、`repo_id`、`branch` 与当前 `scope_nonce` 同时匹配的 `SearchResults`。

## Notes

- `search/query` 是全文搜索链，不替代 Quick Open 的本地文件候选过滤。
- `SearchService` 是可选能力 gate；当前可验收能力不依赖完整 Tantivy 索引已完成。
- 搜索结果必须保持 repo scope 绑定，不能接受过期 request、旧 repo、旧 branch 或旧 scope 返回。
- 搜索读取 ledger 重建内容，而不是直接读取当前磁盘文件文本；当 workspace 文件与 ledger
  存在漂移时，搜索结果以当前 repo ledger projection 为准。
- 当前 `SearchResults` wire payload 只承诺返回 `(doc_id, path, score)`；UI baseline 显示
  文档路径与本地化匹配状态，不承诺正文 snippet 或 query highlight。
- 正文 snippet、命中词高亮和 richer ranking 属于后续协议/UI 扩展，不能作为当前
  `SEARCH-001` 阻塞验收项。

## Chrome MCP Smoke

本 smoke 用于手工验证 `SEARCH-001` 的浏览器路径，不替代单元测试中的 stale
scope / disabled feature 覆盖。

前置条件：

- 本地已安装 `trunk` 与 `wasm32-unknown-unknown` target。
- 默认开发账号为 `admin` / `admin`，仅限 `--dev` 或 `DEVE_ENV=development`。
- 当前 repo 中存在一个可命中的 ledger 文档；默认开发数据可用 `?note` 命中文件名。

步骤：

1. 启动后端：
   `cargo run -p deve_cli --features search --bin deve_cli -- serve --dev --port 3001`
2. 启动前端：
   `NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080`（工作目录为 `apps/web`）
3. 用 Chrome MCP 打开 `http://127.0.0.1:8080/`。
4. 登录 `admin` / `admin`，等待页面显示 `Ready`，并确认 console 出现 WebSocket connected 日志。
5. 点击 Search 入口，输入 `?note`。
6. 等待搜索结果显示 `note.md Full-text match`。
7. 点击结果或按 Enter，确认 Search surface 关闭并打开对应文档。

期望结果：

- 页面经 Trunk 代理连到 `ws://127.0.0.1:8080/ws`。
- 搜索请求返回 repo-scoped `SearchResults`，UI 显示 `Full-text match`。
- 选择结果后打开文档，底部状态仍为 `Ready`。

# repo_open_doc.md - 打开文档操作流示例

## Metadata

- `Flow ID`: `flow.repo.open-doc`
- `Domain`: `repository`
- `Related Feature Chapters`: `docs/features/06_repository.md`, `docs/features/12_commands.md`
- `Related Acceptance Cases`: `CMD-003`, `REPO-FEAT-01`, `STORE-009`

## Operations

### `op.repo.open-doc.open-quick-open`

- `Name`: `Open Quick Open`
- `Surface`: `keyboard-shortcut`
- `Trigger`: `Ctrl/Cmd+P`
- `Preconditions`: 应用主界面已加载，repo scope 已建立
- `Immediate Result`: 文件搜索面板显示
- `Application Entry`: `apps/web/src/components/search_box/`

### `op.repo.open-doc.type-query`

- `Name`: `Type File Query`
- `Surface`: `overlay-input`
- `Trigger`: Quick Open 输入框键入
- `Preconditions`: `op.repo.open-doc.open-quick-open` 已执行
- `Immediate Result`: 根据当前 repo 的文档列表过滤候选项
- `Application Entry`: `apps/web/src/components/search_box/providers/file.rs`

### `op.repo.open-doc.choose-doc`

- `Name`: `Choose Document Result`
- `Surface`: `keyboard-or-pointer`
- `Trigger`: `Enter` 或点击文档结果
- `Preconditions`: 文档候选列表非空
- `Immediate Result`: 选中 `DocId`，关闭搜索面板
- `Application Entry`: `apps/web/src/components/search_box/logic/execute.rs`

### `op.repo.open-doc.request-open`

- `Name`: `Request OpenDoc`
- `Surface`: `editor-state`
- `Trigger`: 当前文档选中且 scope 稳定后自动发送
- `Preconditions`: 目标 `DocId` 在当前 repo 文档列表中，WS 已连接，`scope_nonce` 有效
- `Immediate Result`: 发送 `ClientMessage::OpenDoc`
- `Application Entry`: `apps/web/src/editor/open_scope.rs`, `apps/web/src/editor/hook_open.rs`, `apps/cli/src/server/ws/route/core.rs`

### `op.repo.open-doc.receive-content`

- `Name`: `Receive Document Content`
- `Surface`: `editor`
- `Trigger`: 服务端完成 open doc 响应
- `Preconditions`: `op.repo.open-doc.request-open` 已执行
- `Immediate Result`: 编辑器进入 loading / ready，并显示文档内容
- `Application Entry`: `apps/cli/src/server/ws/route/core.rs`, `apps/cli/src/server/handlers/repo/http.rs`, `crates/core/src/ledger/manager/repository.rs`

## Response Flows

### `op.repo.open-doc.open-quick-open`

1. `User Operation`: 用户按 `Ctrl/Cmd+P` 打开 Quick Open。
2. `Application Response`: 显示文件搜索 UI，准备使用当前 repo 的文档列表作为候选源。
3. `Concrete Modules`: `apps/web/src/components/search_box/`
4. `Core Subsystems`: 无。此步仍是 UI shell。

### `op.repo.open-doc.type-query`

1. `User Operation`: 用户输入文件名或路径片段。
2. `Application Response`: `FileProvider` 用当前 repo 的 `(DocId, path)` 列表做模糊匹配，并生成 `SearchAction::OpenDoc`。
3. `Concrete Modules`: `apps/web/src/components/search_box/providers/file.rs`
4. `Core Subsystems`: `tree`, `ledger`。候选数据来自当前 repo 的文档投影与标识映射。

### `op.repo.open-doc.choose-doc`

1. `User Operation`: 用户按 Enter 或点击某条文档结果。
2. `Application Response`: `execute_action` 调用 `core.on_doc_select`，记录目标 `DocId` 并关闭搜索面板。
3. `Concrete Modules`: `apps/web/src/components/search_box/logic/execute.rs`
4. `Core Subsystems`: 无。此步只提交选择，不直接读取正文。

### `op.repo.open-doc.request-open`

1. `User Operation`: 用户已完成选中文档，系统进入实际打开阶段。
2. `Application Response`: 前端检查 `doc_selected`、repo scope、branch/repo switch idle、WS connected、`scope_nonce`，满足后发送 `ClientMessage::OpenDoc`。
3. `Concrete Modules`: `apps/web/src/editor/open_scope.rs`, `apps/web/src/editor/hook_open.rs`, `apps/cli/src/server/ws/route/core.rs`
4. `Core Subsystems`: `protocol`, `tree`

### `op.repo.open-doc.receive-content`

1. `User Operation`: 用户等待文档载入并看到正文。
2. `Application Response`: 服务端校验 scope nonce，执行 open doc 处理；repo 侧按 `RepoSelector + DocId` 解析当前仓库，读取 ledger ops，并重建文本内容返回给编辑器。
3. `Concrete Modules`: `apps/cli/src/server/ws/route/core.rs`, `apps/cli/src/server/handlers/repo/http.rs`, `crates/core/src/ledger/manager/repository.rs`, `crates/core/src/ledger/metadata.rs`, `crates/core/src/state.rs`
4. `Core Subsystems`: `protocol`, `ledger`, `tree`

## Notes

- `Quick Open` 是入口容器，不是第一层节点；第一层应是 `open-quick-open`、`type-query`、`choose-doc`、`request-open`、`receive-content`。
- 当前 Quick Open 是用户概念；真实入口是 `UnifiedSearch` 空模式与 `FileProvider`，代码位于 `apps/web/src/components/search_box/`。
- 文档列表与正文读取不是同一操作：列表更接近 `list docs`，正文读取更接近 `OpenDoc`。
- Quick Open 进入哪一类 provider、以及结果如何先被统一 `SearchAction` 路由，已单独建模在 `command_surface_mode_routing.md` 与 `command_surface_action_routing.md`。
- `/api/repo/docs` 适合支撑候选列表；真正的文档打开主链路以 WS `OpenDoc` 为中心。
- `DocId` 是打开动作的权威键，路径只应作为检索与展示辅助。

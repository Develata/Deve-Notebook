# Deve-Note 代码实现状态与功能映射 (Implementation Status & Plan Mapping)

本文档以文件树形式展示当前代码库的详细实现逻辑，并映射到架构规划文档中的具体章节。

**Plan Mapping Key:**
- `[Arch]`: 01_architecture.md (核心架构)
- `[UI-Arch]`: 03_ui_architecture.md (UI 架构)
- `[Backend]`: 04_backend.md (后端架构)
- `[Data]`: 05_data_flows.md (数据流)
- `[Runtime]`: 07_runtime_ops.md (运行时与插件)

---

## 📂 crates/core (核心库)

实现了 **Trinity Isolation** 和 **P2P Sync** 的核心逻辑。

- **`src/`**
  - **`config.rs`**: **配置管理** `[Runtime]`
    - **逻辑**: 使用 `std::env` 加载环境变量，实现 `SyncMode` (Auto/Manual) 和 `AppProfile` (Standard/LowSpec) 的 `FromStr` trait，利用 `serde` 进行序列化。
  - **`error.rs`**: **统一错误处理** `[Arch]`
    - **逻辑**: 基于 `thiserror` 定义 `AppError`，统一处理 IO、Redb、Codec 和 Plugin 错误。
  - **`models.rs`**: **基础数据模型** `[Data]`
    - **逻辑**: 
      - `DocId`/`PeerId`: 封装 UUID V4。
      - `VersionVector`: 实现为 `BTreeMap<PeerId, u64>`，提供因果顺序比较 (`PartialOrd`)。
  - **`protocol.rs`**: **通信协议** `[Backend]`
    - **逻辑**: 定义 `serde` 可序列化的 `ClientMessage` (Create, Edit...) 和 `ServerMessage` (NewOp, Snapshot...) 枚举，作为 WebSocket 通信载荷。
  - **`state.rs`**: **CRDT 状态机** `[Data]`
    - **逻辑**: 
      - `reconstruct_content(ops)`: 拓扑排序 Op 依赖关系 (DAG)，线性化生成最终文本。
      - `compute_diff(old, new)`: 使用 Myers 差分算法计算文本变更，生成新的 `Op`。
  - **`watcher.rs`**: **文件系统监听** `[Backend]`
    - **逻辑**: 封装 `notify-debouncer-mini`，在独立线程中运行，过滤非 `.md` 文件和 `.git` 目录，防抖窗口 200ms。
  - **`vfs.rs`**: **虚拟文件系统 (VFS)** `[Backend]`
    - **逻辑**:
      - `get_inode`: 使用平台特定 API (Windows `file_index`) 获取文件唯一标识，经 `StableHasher` (FNV-1a) 映射为稳定 `u128`，解决文件重命名检测问题。
      - `scan`: 遍历 `WalkDir`，对比磁盘文件与 Ledger 记录，自动 CRUD 以保持一致性。
  - **`ledger/`** `[Backend: Repository Manager]`
    - **`mod.rs`**: **RepoManager**
      - **逻辑**: 封装 `Redb` 事务。`append_local_op` 负责通过 VFS 写入磁盘并更新 DB；`append_remote_op` 仅更新 DB (Shadow Repo 模式)。
    - **`store.rs`**: **存储后端**
      - **逻辑**: 定义 Redb 表：`DOCS` (Path->DocId), `OPS` (DocId->Vec<Op>), `SYNC_STATE` (PeerId->Vector)。实现原子读写。
    - **`ops.rs`**: **CRDT 操作**
      - **逻辑**: 定义 `Op` 结构 (Seq, Deps, Content)。
    - **`snapshot.rs`**: **快照管理**
      - **逻辑**: 每 N 个 Op 生成一次全量文本快照，存入 `SNAPSHOTS` 表。查询时优先加载最近快照 + 后续 Ops。
    - **`shadow/`**: **影子仓库 (Trinity Isolation)** `[Arch: Trinity Isolation]`
      - **逻辑**: 实现 `ShadowRepo` trait，允许并在同一 DB 中存储多个对等点的视图 (Shadows)，互不干扰，仅通过 `Merge` 操作交换数据。
  - **`sync/`** `[Backend: Gossip Protocol]`
    - **`engine.rs`**: **同步引擎**
      - **逻辑**: 
        - 比较本地与远程 `VersionVector`。
        - *Push*: 找出本地有但远程没有的 Ops。
        - *Pull*: 处理远程发来的 Ops，存入 `OpBuffer`。
    - **`buffer.rs`**: **因果缓冲** `[Backend: Reconciliation]`
      - **逻辑**: 暂存接收到的乱序 Ops。当 Op 依赖的所有前驱 Op 都存在时，才应用该 Op。
  - **`plugin/`** `[Runtime: Dual-Engine]`
    - **`runtime.rs`**: **Wasm 运行时**
      - **逻辑**: 集成 `wasmtime`，配置资源限制 (Fuel)。注入 Host Functions (如 `host_log`, `get_doc`) 供插件沙箱调用。
  - **`utils/hash.rs`**: **稳定哈希**
    - **逻辑**: 实现 FNV-1a 算法，确保跨进程重启后内存对象的 Hash 值一致 (用于 Inode 映射)。

## 📂 apps/cli (后端服务)

实现了 **Server-Side Logic** 和 **WebSocket Gateway**。

- **`src/`**
  - **`main.rs`**: **CLI 入口** `[Runtime]`
    - **逻辑**: 使用 `clap` 解析 `serve`, `scan`, `init` 子命令。初始化 `tracing-subscriber` 进行结构化日志记录。
  - **`commands/serve.rs`**: **服务引导** `[Backend]`
    - **逻辑**: 构建依赖注入容器 (AppState: RepoManager + SyncManager)。启动 `Axum` HTTP Router，挂载 `/ws` 端点。
  - **`server/ws.rs`**: **WebSocket 网关** `[Backend]`
    - **逻辑**: 
      - **连接管理**: 为每个连接分配临时 `PeerId`。
      - **消息路由**: 解析 JSON -> `ClientMessage` -> 分发给 Handler。
      - **通道模型**: 使用 `Broadcast` (全量推送) 和 `MPSC` (单播响应) 通道组合。
  - **`server/handlers/`**
    - **`document.rs`**: **OT/CRDT 协作** `[Data: Flows]`
      - **逻辑**: 处理 `Edit` 消息。调用 `RepoManager` 持久化 Op，并通过广播通道转发给其他客户端。
    - **`sync.rs`**: **P2P 同步处理** `[Backend: Gossip]`
      - **逻辑**: 处理 `SyncHello` 握手。调用 `SyncEngine` 生成差异补丁 (`SyncPush/Resp`)。
    - **`merge.rs`**: **手动合并控制** `[Data: P2P Merge]`
      - **逻辑**: 处理 `SetSyncMode` (切换自动/手动)。在手动模式下，将接收到的 Ops 放入暂存区而非直接应用，直到收到 `ConfirmMerge`。
    - **`system.rs`**: **系统状态** `[UI-Arch: Branch Switcher]`
      - **逻辑**: 响应 `ListShadows`，列出所有已知的远程 Peer 及其版本状态，供前端分支切换器使用。

## 📂 apps/web (Web 前端)

实现了 **UI Architecture** 和 **Cockpit Design**。

- **`src/`**
  - **`app.rs`**: **应用架构** `[UI-Arch]`
    - **逻辑**: 
      - **Layout**: CSS Grid 实现 "ActivityBar (Fixed) | Sidebar (Resizable) | Editor (Flex)" 布局。
      - **Context**: 根级提供 `Locale` 和 `WsService`。
  - **`hooks/use_core.rs`**: **前端状态中枢** `[UI-Arch: Data Flow]`
    - **逻辑**: 
      - 维护响应式信号 (`docs`, `current_doc`, `stats`)。
      - 统一管理 WebSocket 发送 (`ws.send`)。
      - 集中处理 WebSocket 接收 (`ServerMessage::match`) 并更新信号。
  - **`components/sidebar/`** `[UI-Arch: Component System]`
    - **`tree.rs`**: **文件树算法**
      - **逻辑**: 将扁平的路径列表 (`Vec<String>`) 转换为嵌套的 `FileNode` 树结构。使用递归构建目录层级。
    - **`explorer.rs`**: **资源管理器**
      - **逻辑**: 渲染 `FileTreeItem`。实现右键上下文菜单 (`ContextMenu`) 状态管理。
    - **`source_control.rs`**: **版本控制面板** `[UI-Arch: Branch Switcher]`
      - **逻辑**: 
        - 订阅 `core.pending_ops` 显示待合并变更。
        - 实现 `Time Travel` 滑块：通过 `playback_version` 信号控制编辑器视图回滚。
  - **`editor/`** `[UI-Arch: Editor Kernel]`
    - **`hook.rs`**: **CodeMirror 集成**
      - **逻辑**: 使用 `use_editor` 自定义 Hook 管理 JS 编辑器实例生命周期。
      - **同步**: 监听 `core.current_doc` 变更，触发 `OpenDoc`。处理 `NewOp` 消息，调用 `ffi::applyRemoteOp` 更新编辑器内容。
    - **`playback.rs`**: **客户端回放** `[Data: History]`
      - **逻辑**: 纯客户端实现的 CRDT 重构。给定一组 Ops 和目标版本号，在内存中重建该版本的文本内容。
    - **`ffi.rs`**: **Wasm Bindings**
      - **逻辑**: 定义 `extern "C"` 接口，通过 `wasm-bindgen` 调用 `adapter.js` 中的 CodeMirror API。
  - **`api/connection.rs`**: **连接韧性** `[Data: Offline]`
    - **逻辑**: 实现指数退避重连算法。维护 `ConnectionStatus` (Connected, Reconnecting, Offline) 信号。

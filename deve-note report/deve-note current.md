# Deve-Note 代码实现状态与功能映射 (Implementation Status & Plan Mapping)

本文档以文件树形式展示当前 code base 的详细实现逻辑，并映射到架构规划文档中的具体章节。

**Plan Mapping Key:**
- `[Arch]`: 01_terminology.md / 02_positioning.md
- `[Store]`: 04_storage.md
- `[Repo]`: 06_repository.md
- `[Diff]`: 07_diff_logic.md
- `[UI]`: 08_ui_design.md
- `[Auth]`: 09_auth.md
- `[Plugins]`: 11_plugins.md
- `[Cmd]`: 12_commands.md
- `[Conf]`: 13_settings.md
- `[Stack]`: 14_tech_stack.md

---

## 🛑 差异与冲突 (Discrepancies & Conflicts)

以下列出当前代码实现与规划文档 (`deve-note plan/`) 不一致的地方：

1.  **Ledger 模块结构**:
    *   **Plan/Old Doc**: 提及 `ledger/store.rs` 作为存储后端。
    *   **Current Code**: 实际为 `ledger/schema.rs` 定义表结构 (`DOCID_TO_PATH`, `LEDGER_OPS` 等)，逻辑分散在 `mod.rs` 和子模块中。
2.  **Snapshot 表结构**:
    *   **Plan**: 提及单表 `SNAPSHOTS`。
    *   **Current Code**: 实际使用双表结构 `SNAPSHOT_INDEX` (Index) 和 `SNAPSHOT_DATA` (Blob) 以优化性能。
3.  **Graph 依赖**:
    *   **Plan**: `14_tech_stack.md` 提及 `Pixi.js` (Web) / `Cosmic` (Rust)。
    *   **Current Code**: `apps/web/Cargo.toml` (未完全验证) 或 `src/app.rs` 中尚未发现显式的 Graph 视图实现代码或引用。
4.  **Merge Logic (关键缺失)**:
    *   **Plan**: `07_diff_logic.md` 明确要求 Atomic Persistence 和 3-Way Merge。
    *   **Current Code**: `ledger/merge.rs` 仅有占位符实现，**严重缺失**。需优先解决。

---

## 📂 crates/core (核心库)

实现了 **Trinity Isolation** 和 **P2P Sync** 的核心逻辑。

- **`src/`**
  - **`config.rs`**: **配置管理** `[Conf]`
    - **逻辑**: 遵循 12-Factor，优先加载 `DEVE_PROFILE`, `DEVE_SYNC_MODE`.
    - **实现**: `Config::load()` 处理 `Standard`/`LowSpec` 预设和 `Auto`/`Manual` 同步模式。
  - **`error.rs`**: **统一错误处理** `[Arch]`
    - **逻辑**: 基于 `thiserror` 定义 `AppError`。
  - **`models.rs`**: **基础数据模型** `[Store]`
    - **逻辑**: 定义 `DocId`, `PeerId`, `VersionVector` (BTreeMap 实现).
  - **`protocol.rs`**: **通信协议** `[Network]`
    - **逻辑**: 定义 WebSocket 载荷 `ClientMessage` / `ServerMessage`。
  - **`state.rs`**: **CRDT 状态机** `[Diff]`
    - **逻辑**: `compute_diff` (Myers) 和 `reconstruct_content` (DAG 线性化).
  - **`watcher.rs`**: **文件系统监听** `[Repo]`
    - **逻辑**: 使用 `notify-debouncer-mini` 监听 Vault 变更。
  - **`vfs.rs`**: **虚拟文件系统** `[Repo]`
    - **逻辑**: 处理 Inode 映射 (FNV-1a hash) 防止文件重命名丢失追踪。
  - **`ledger/`** `[Repo: Repository Manager]`
    - **`mod.rs`**: **Manager 入口**
      - **逻辑**: 管理 `local_db` (Store B) 和 `shadow_dbs` (Store C)。提供 `append_local_op` 等核心 API。
    - **`schema.rs`**: **Redb 表定义** `[Store]`
      - **逻辑**: 定义 `DOCID_TO_PATH`, `PATH_TO_DOCID`, `LEDGER_OPS`, `SNAPSHOT_INDEX` 等表。
    - **`ops.rs`**: **Op 读写**
      - **逻辑**: 封装对 `LEDGER_OPS` 表的原子读写。
    - **`snapshot.rs`**: **快照管理** `[Store]`
      - **逻辑**: 维护 `snapshot_depth`，写入快照数据。
    - **`source_control.rs`**: **版本控制** `[Repo]`
      - **逻辑**: 实现 `stage_file`, `create_commit`, `list_staged` 等类 Git 操作。
    - **`shadow/`**: **影子库实现**
      - **逻辑**: 管理远端 Peer 的独立数据库文件 (`remotes/*.redb`)。
  - **`sync/`** `[Network: Gossip]`
    - **`engine.rs`**: **同步引擎**
      - **逻辑**: 计算 VersionVector 差异，生成 Push/Pull 任务。
    - **`buffer.rs`**: **因果缓冲**
      - **逻辑**: 解决乱序 Op 问题 (`OpBuffer`).
  - **`plugin/`** `[Plugins]`
    - **`runtime.rs`**: **Rhai/Wasm 运行时**
      - **逻辑**: 集成 `rhai` (根据 Cargo.toml) 或 WASM 运行时 (代码中提及 `wasmtime` 但 `Cargo.toml` 只有 `rhai`?). *注: Cargo.toml 仅显示 rhai, verify required.*

## 📂 apps/cli (后端服务)

实现了 **Server-Side Logic** 和 **WebSocket Gateway**。

- **`src/`**
  - **`main.rs`**: **CLI 入口** `[Cmd]`
    - **逻辑**: `clap` 解析 `serve`, `scan`, `init`, `watch` 等命令。
  - **`commands/`**: **命令实现**
    - **`serve.rs`**: 启动 Axum Server `[Network]`.
    - **`scan.rs`**: 执行全量索引扫描 `[Repo]`.
  - **`server/`**
    - **`ws.rs`**: **WebSocket 网关** `[Network]`
      - **逻辑**: 处理连接生命周期，PeerId 分配，消息路由 (Broadcast/MPSC)。
    - **`handlers/`**: **消息处理器**
      - **`document.rs`**: 处理 `Edit`, `Open` 等协作消息。
      - **`sync.rs`**: 处理 `SyncHello`, `SyncPush`。

## 📂 apps/web (Web 前端)

实现了 **UI Architecture** 和 **Cockpit Design**。
*注: 基于 Leptos v0.7 + Tailwind CSS*

- **`src/`**
  - **`app.rs`**: **应用架构** `[UI]`
    - **逻辑**: 定义 Grid 布局 (ActivityBar | Sidebar | Editor)。
    - **Context**: 提供 `Locale`, `SearchControl`.
  - **`hooks/use_core.rs`**: **状态中枢** `[UI: Data Flow]`
    - **逻辑**: 封装 WebSocket `send`/`recv`，驱动响应式信号 `docs`, `current_doc`.
  - **`components/`**
    - **`activity_bar.rs`**: 左侧一级导航。
    - **`sidebar/`**: 二级侧边栏 (Explorer, SourceControl)。
    - **`search_box/`**: **Unified Search** `[UI: Modal]` (Cmd+P).
    - **`editor/`**: **CodeMirror 集成** `[UI: Rendering]`
      - **`hook.rs`**: 通过 Wasm Bindings 调用 JS 编辑器。
    - **`diff_view.rs`**: **Diff 视图** `[Diff]`.

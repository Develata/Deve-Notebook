# 03_storage/repair.md - Repair Runtime

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-10`
- `Parent`: `03_storage/index`
- `Primary Code Areas`: `crates/core/src/sync/projection_repair_runtime.rs`, `crates/core/src/ledger/manager/repair_runtime.rs`, `crates/core/src/ledger/snapshot.rs`, `apps/cli/src/commands/export/`

> 本文件是 `03_storage` 章的 `repair_runtime` 子合同：browser recovery semantics、workspace/ledger/catalog repair、backup/export 与 degraded mode 边界。章节骨架与总览见 [index.md](./index.md)。

## 3. Physical Layout（repair 部分）

> §3.4 Browser Storage Layering 与 §3.4.1 Trust Registration Flow 见 [index.md#browser-storage-layering](./index.md#browser-storage-layering)。

### 3.4.2 Recovery Semantics

- `Cookie 可用 + IndexedDB 可用 + WebCrypto 可用`
  - 正常进入 repo-scoped sync/runtime。
- `Cookie 可用，但 IndexedDB 不可用`
  - 进入 `DegradedSyncMode`。
  - UI **MUST** 只读。
  - 禁止 `RegisterWriter`、`SyncPush`、pending write enqueue、repo-scoped durable cache。
- `Cookie 可用，IndexedDB 可用，但 WebCrypto key 缺失`
  - 必须重新生成 repo-scoped key 并重新注册 browser peer。
  - 旧 browser peer identity 与旧 cache **MUST** 视为不可恢复。
- 站点数据被清理
  - 浏览器 **MUST** 视为新 light peer。
  - 任何旧的 peer metadata、pending browser cache、repo-scoped trust state 都不得被猜测恢复。

## 9. Recovery / Repair

### 9.1 Workspace Recovery

- Projection Workspace 损坏时，从 ledger + snapshot 重建 projection。
- 无法解释的 workspace 偏差视为状态漂移，必须 reconcile 或 hard rebuild。

### 9.2 Ledger Repair Boundary

- 只有显式 repair / reset 流程才允许从 Projection Workspace 反向导入生成新 ledger。
- 日常运行路径不得把 Projection Workspace 当成 authority fallback。

### 9.3 Catalog / Runtime Repair

- 允许修复 local/remote repo catalog、runtime tables、source control side tables。
- repair 不得伪造 authority history。

### 9.4 Backup / Export {#backup-export}

- repo **MAY** 定期生成只读 backup snapshot。
- 系统 **MUST** 支持将 ledger 导出为 JSON Lines。
- schema v2 repo 只能由离线命令 `deve export --allow-legacy-v2 --format json|markdown` 通过专用兼容读取器访问；普通 RepoManager、serve、write、sync 与 repair mutation 必须在 schema gate fail-closed。
- 兼容读取器只能打开用户明确选择的既有 v2 `.redb` 并启动 read transaction，不创建数据库、不写 metadata/table、不获取 writer authority。运行中的 server 持锁时必须拒绝。
- JSON Lines 必须按旧 `GlobalSeq` 单调导出原始 v2 `event / peer_id / seq / client ids` 并显式标记 `legacy_schema_version=2`；不得把旧 peer/seq 投影成 v3 origin。
- Markdown 导出只允许 fold v2 content/structure facts 生成当前可读投影；structure authority 不一致时 fail-closed，不使用 path metadata 猜测修复。
- v2 导出后必须重建 v3 repo，并通过受控 import/reconcile 生成当前 host identity 下的新事实；不得把旧 actor 标签猜测成物理 peer 或原地伪造连续序列。

### 9.5 Hard Failure vs Degraded Mode

- 以下情况允许进入 degraded mode：
  - projection locator 缺失、不可访问或冲突
  - projection 缓存损坏
  - watcher overflow 待 reconcile
  - workspace writeback 失败但 ledger 已提交
  - 浏览器 light peer 的 durable storage 缺失，但 session 仍可用
- 以下情况 **MUST** hard fail / quarantine：
  - authority table 损坏且无法验证 append order
  - repo identity / catalog 冲突无法唯一解析
  - repair 过程检测到 history 自相矛盾

`DegradedSyncMode` 规则：

- 只适用于浏览器 light peer 的 storage/runtime 缺失场景。
- 允许 session 存在。
- 不允许 authority write、pending write、`RegisterWriter`、`SyncPush`。
- 必须显式暴露给 network/runtime 层，不得伪装成完整 online writable 状态。

## 10. Forbidden Patterns（repair）

> 跨层禁止项见 [index.md](./index.md)。

- 让 side table 或 snapshot 成为删除真源。

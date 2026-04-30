# 04_storage.md - Ledger、Projection 与 Workspace 存储工程蓝图

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Counterpart Feature`: `docs/features/04_storage.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/07_storage_repo.md`

## 1. Scope

本章定义：

- Repo authority 如何持久化到 ledger。
- Workspace、Tree、Snapshot、Source Control side tables 如何作为 projection 或 workflow side tables 存在。
- Watcher、projection writeback、repair、backup 如何在存储层协作。

本章不描述按钮、列表、提示语或用户操作示例；这些属于 `docs/features/04_storage.md`。

## 2. Authoritative Entities

### 2.1 Core Stores

- `Store A: Vault`
  - `vault/<repo_name>/`
  - 是 workspace projection 的物理容器，不是 authority。
- `Store B: Local Branch Ledger`
  - `ledger/local/*.redb`
  - 本地唯一可写 authority。
- `Store C: Remote Branch Ledgers`
  - `ledger/remotes/<peer>/*.redb`
  - 远端镜像 authority，仅同步路径可写。

### 2.2 Authority Model

对任意 repo `r`：

```text
L_r = OrderedLog<LedgerEntry>
S_r = Fold(L_r)
P_r = Project(S_r)
Workspace_r = P_r ⊕ D_r
```

其中：

- `L_r`：唯一权威事实日志
- `S_r`：由 ledger fold 得到的逻辑状态
- `P_r`：规范 projection
- `D_r`：已被系统跟踪的 workspace 偏差

### 2.3 Facts Partition {#facts-partition}

- `Content Facts`
  - 面向 `DocId`
  - Insert / Delete / replace 等文本变化
- `Structure Facts`
  - 面向 `NodeId / DocId`
  - Create / Rename / Move / Delete 等结构变化

### 2.4 Non-Authoritative Runtime State

- `pending_fs_ops`
- `staging`
- `commit_index`
- `snapshot_index`
- `tree cache`
- `path_cache`
- `NodeMeta projection`

这些都是 workflow state、projection state 或 performance state，**MUST NOT** 升格为 authority。

### 2.5 Clean File Policy

- `Zero Injection`
  - 系统 **MUST NOT** 向用户 Markdown 文件注入系统元数据。
  - 严禁把 `DocId`、`NodeId`、creation time、repo id、peer metadata 写进 YAML Frontmatter、HTML comment 或隐藏 sidecar markdown。
- `Metadata Source`
  - 用户手写 Frontmatter 只能被视为文档 payload。
  - Frontmatter 的存在、缺失、修改都 **MUST NOT** 反向改变 ledger/system metadata。
- `No Hidden Shadow Files`
  - authority 恢复不得依赖同目录的额外 markdown metadata 文件。
  - repo runtime 元数据只能进入 `.notegit/` 或 runtime side tables。

## 3. Physical Layout

### 3.1 Ledger Layout

- `ledger/local/<repo_name>.redb`
- `ledger/remotes/<peer_name>/<repo_name>.redb`
- `ledger/.host/identity.key`
- `ledger/backups/<repo_name>-<timestamp>.redb`

### 3.2 Repo Runtime Layout {#repo-runtime-layout}

- `vault/<repo_name>/.notegit/`
  - repo keys
  - pending/staged side tables
  - commit/runtime metadata
  - migration archives

约束：

- `.notegit/` **MUST** 被 watcher 忽略。
- `.notegit/` 可以随 repo 备份，但 **MUST NOT** 被跨 repo 复用。
- `.notegit/` 是 Deve-owned repo runtime 目录，当前继续保留该命名。

### 3.2.1 Git 镜像边界 {#git-ecosystem-coexistence}

Deve 支持 `.notegit/` 与 `.git/` 在同一个 repo 工作区中共存，但二者职责必须分离：

- `.notegit/` 是 Deve-owned runtime 目录，保存 ledger-aware workflow state 与必要 side table。
- `.git/` 是 Git ecosystem mirror 目录，只用于复用 Git 工具链、远程托管、审计、备份与发布生态。
- repo-local `.gitignore` **MUST** 忽略 `.notegit/`，避免 Git mirror 泄漏 Deve runtime state。
- watcher、scan、sync 与 projection rebuild **MUST** 按路径段语义忽略 `.notegit/` 与 `.git/` 内部路径。
- Deve core **MUST NOT** 使用 `.git/` 作为 repo authority、runtime side table 或 hidden metadata 目录。
- `.git/` 不得参与 ledger fold、repo scope 解析、stage/commit authority 或 repair。

Git mirror 是一等生态桥接层，而不是 authority 替代品：

- Deve ledger commit 成功并持久化 projection 后，系统 **MAY** 追加 `GitMirrorQueued`。
- Git mirror update **MAY** 以 Deve commit/projection 为输入生成 Git commit，并记录 `Deve-Commit-Id`、`ledger_seq`、`repo_id` 等映射。
- Git mirror 失败 **MUST NOT** 回滚已成功的 Deve ledger commit；系统 **MUST** 标记 `GitMirrorOutOfSync`，并提供 retry / repair / status 可观测路径。
- 外部 `git checkout/reset/pull/rebase` 造成的工作区变化只能进入 pending/import 路径，**MUST NOT** 自动反向改写 ledger authority。
- Git import **MUST** 只进入 pending/import；只有后续 Deve stage/commit 才能生成 ledger facts。
- Git export 与 Git push **MUST NOT** 反向改写 ledger authority、`StagedEntry` 或 `.notegit/`。
- Git push **MUST** fail-closed 于脏 Deve Source Control、脏 Git worktree、未导出 mirror record、out-of-sync mirror record、`.notegit` 泄漏或 Git HEAD 未映射到最新 Deve commit。
- Web 或后台任务 **MUST NOT** 隐式执行 Git 写操作；任何 Git 写入都必须来自显式命令或后续单独设计的确认式执行面。

### 3.3 Collision Rules

- 同一 branch 下，同名但不同 repo identity 的 `.redb` 文件 **MUST** 自动重命名。
- 物理文件名冲突不得改变逻辑 repo identity。

### 3.4 Browser Storage Layering {#browser-storage-layering}

浏览器端分层存储属于本章约束的一部分：

- `localStorage`
  - 纯 UI 偏好
- `IndexedDB`
  - repo-scoped metadata、vector、cache metadata
- `WebCrypto`
  - 私钥材料
- authority 业务数据
  - 仍在 server ledger，不在浏览器长期持有

### 3.4.1 Trust Registration Flow

浏览器 repo-scoped trust registration 必须按以下顺序进行：

1. 先确认 user session 已通过 `/api/auth/me` 或 ws 入口鉴权。
2. 读取 IndexedDB 中该 `repo_id` 的 browser peer metadata。
3. 若 metadata 不存在，则用 `WebCrypto` 生成 repo-scoped keypair，私钥必须 `extractable: false`。
4. 把 peer public metadata 写入 IndexedDB；私钥材料仅存于 WebCrypto。
5. 发送 repo-scoped `SyncHello` 完成 browser peer 注册。
6. 只有当 session、IndexedDB metadata、repo key 三者都齐备时，浏览器才允许进入可同步写态。

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

### 3.5 Internal Path Normalization {#internal-path-normalization}

- 所有持久化到 ledger、projection table、side table、sync payload 的路径字符串 **MUST** 统一为 forward slash。
- 规范化边界：
  - 进入系统：watcher、file dialog、CLI path 参数进入 authority/runtime 前，必须调用 `to_forward_slash`。
  - 离开系统：仅在直接调用 OS 文件系统 API 的瞬间，才允许转换回 native separator。
- 禁止：
  - 在不同表中混用 `\` 与 `/`
  - 依赖 display path 作为 authority key
  - 通过字符串替换拼接路径身份

## 4. Storage Tables and Indexes

### 4.1 Core Tables

- `LEDGER_OPS: GlobalSeq -> LedgerEntry`
- `DOC_OPS: DocId -> [GlobalSeq]`
- `NODEID_TO_META: NodeId -> NodeMeta`
- `PATH_TO_NODEID: Path -> NodeId`
- `INODE_TO_NODEID: Inode -> NodeId`
- `SNAPSHOT_INDEX: DocId -> [SeqNo]`
- `SNAPSHOT_DATA: SeqNo -> ContentBlob`

### 4.2 Sequence Contract

- `GlobalSeq`
  - repo 范围内全序
  - `LEDGER_OPS` 主键
- `PEER_DOC_SEQ`
  - `(DocId, PeerId) -> u64`
  - per-doc per-peer 单调计数

规则：

- `GlobalSeq` 决定落盘顺序。
- `PEER_DOC_SEQ` 只作为 entry metadata，不得替代全序主键。

### 4.3 Runtime Side Tables and Repo Metadata

- `.notegit/` 或 repo runtime state 中至少要存在以下 side tables / metadata：
  - `pending_fs_ops`
    - key: stable pending op id
    - value: normalized path, op kind, inode/file id, content hash/base hash, detected_at
  - `staging`
    - key: staged entry id
    - value: pending source ref, staged_at, stage actor, repo head anchor
  - `commit_index`
    - key: commit id / anchor
    - value: ledger head, commit meta, affected docs/nodes
  - `projection_runtime`
    - value: last_projection_seq, last_projection_hash, degraded flag, repair marker
  - `watcher_runtime`
    - value: suppressor fingerprints, overflow marker, last_full_scan_at
- 这些 side tables **MUST** 明确标记为 workflow/runtime state，不得被上层误当成 authority state。

### 4.4 Snapshot Storage Contract

- snapshot 采用 dual-table：
  - `SNAPSHOT_INDEX`
    - `DocId -> [SeqNo]`
  - `SNAPSHOT_DATA`
    - `SeqNo -> ContentBlob`
- 规则：
  - snapshot 永远锚定到已确认 `GlobalSeq`
  - snapshot pruning **MUST** 只删除已被 `SNAPSHOT_INDEX` 脱链的旧快照
  - pruning 不得删除当前 head、最近检查点和正在被 restore 使用的快照

## 5. State Machines

### 5.1 Repo Mount Lifecycle

```text
RepoDiscovered
  -> RepoOpened
  -> RuntimeTablesReady
  -> ProjectionReady
  -> WatcherReady
  -> Mounted
```

约束：

- `WatcherReady` 是打开 repo 的最后一步。
- watcher 初始化失败 **MUST** fail-closed。

### 5.2 Write Lifecycle

```text
Intent
  -> AppendValidated
  -> LedgerCommitted
  -> ProjectionRebuilt
  -> WorkspacePersisted
```

允许的失败旁路：

```text
LedgerCommitted -> ProjectionWritebackFailed -> RecoverableProjectionFault
```

逐状态约束：

- `Intent`
  - trigger: editor save / cli command / source control apply
  - guard: repo mounted, auth/session valid, append precheck passed
- `AppendValidated`
  - effect: content/structure facts serialized and assigned candidate ordering
  - failure: validation reject -> no workspace writeback
- `LedgerCommitted`
  - effect: `GlobalSeq` assigned, `LEDGER_OPS` durable
  - invariant: success 后 authority 已成立
- `ProjectionRebuilt`
  - effect: tree/path/meta/snapshot updated
- `WorkspacePersisted`
  - effect: vault matches projection or explicit dirty overlay
- `RecoverableProjectionFault`
  - effect: repo enters degraded-but-authority-valid state, waiting for rebuild

### 5.3 External Edit Lifecycle

```text
FsEvent
  -> Debounced
  -> PendingFsRecorded
  -> Staged
  -> LedgerCommitted
  -> Cleared
```

补充：

- `FsEvent`
  - source: notify backend / FileObserver / kqueue equivalent
- `Debounced`
  - duplicate burst events coalesce by path + final content/inode state
- `PendingFsRecorded`
  - effect: side table row inserted/updated, never authority mutation
- `Staged`
  - effect: explicit user confirmation only
- `Cleared`
  - effect: consumed by commit or discarded by reset-to-projection

## 6. Ledger-First Write Paths

### 6.1 Path A: Controlled Editor / CLI Writes

1. 生成写入意图。
2. 校验 auth、repo binding、writer identity、append validity。
3. 生成 `Content Facts` / `Structure Facts`。
4. 追加到 ledger。
5. 重建或增量更新 projection。
6. 持久化回 workspace。

规则：

- **MUST NOT** 先改 Vault 再补 ledger。

### 6.2 Path B: Watcher / External Edit Ingestion

1. watcher 捕获文件系统事件。
2. 经 debounce、路径归一化、`.deveignore` / internal path 过滤、inode 解析后写入 `pending_fs_ops`。
3. 非文档目录事件只允许触发 repo-scoped scan；scan 必须复用同一套忽略规则。
4. 仅暴露 working directory 偏差，不改变 authority。

规则：

- watcher 事件 **MUST NOT** 直接写 ledger。
- delete / rename / move 必须先成为候选，再经 Stage / Commit 进入结构事实。
- 被忽略路径 **MUST NOT** 通过 watcher/scan 反向摄入到 `pending_fs_ops`、tree projection 或 ledger。

### 6.3 Path C: Stage -> Commit

1. 从 `pending_fs_ops` 迁移到 `staging`。
2. 以当前 confirmed projection 为 base 计算差异。
3. 生成 content / structure facts。
4. 追加到 ledger。
5. 写 commit anchor。
6. 清理 side tables。
7. 回写 projection。

额外约束：

- stage 是真实迁移，不是 UI 布尔标记。
- commit 生成 diff 时 base **MUST** 是当前 confirmed projection，而不是当前 vault 内容快照。
- discard 的语义只能是“恢复 vault 到 projection + 清理 pending/staging”，不得触碰 ledger history。

## 7. Projection and Persistence Contract {#projection-contract}

- 所有系统写盘都必须满足：

```text
Intent -> Ledger Facts -> Projection -> Vault
```

- `metadata`、`path mapping`、`tree cache`、`NodeMeta` 只能由 projection builder 写入。
- handler、component、source control action 不得把这些表当成主写路径。
- ledger append 成功而 projection 失败时，系统 **MUST** 标记 recoverable fault，并支持从 ledger 重建。

具体要求：

- projection builder 至少输出：
  - `NodeMeta`
  - path mapping
  - doc projection
  - tree projection
  - snapshot checkpoints
- projection writeback **MUST** 具备 repo-scoped 原子感知：
  - 同一 repo 的 projection rebuild 与 workspace persist 不能并发乱序覆盖
  - 不同 repo 之间必须隔离
- `PersistGuard` / `WriteSuppressor` 必须在 projection writeback 前后成对生效，防止 watcher storm

## 8. Watcher Contract {#watcher-contract}

### 8.1 Backend Abstraction

- 必须存在统一 `FsWatcherBackend` trait。
- Desktop / Android / iOS 后端必须在后端层归一化事件语义。

### 8.2 Startup Semantics

- watcher_start 是 repo open 的最后一步。
- 启动前必须执行一次全量 scan。
- 启动扫描 **MUST** 读取 vault 根目录 `.deveignore`，并在创建 pending candidate 前跳过被忽略的 Markdown。
- scan 与 watcher 首批事件之间的去重必须由 side table 幂等性保证。

### 8.3 忽略与路径过滤

- `.deveignore` 位于 vault 根目录；直接 watcher 事件、目录重扫与启动扫描 **MUST** 使用同一套匹配语义。
- 忽略匹配 **MUST** 同时接受 vault-relative path（`<repo>/<path>`）与 repo-relative path（`<path>`），保证 repo-local 与 vault-wide 规则语义稳定。
- `.notegit/` 与其它 repo 内部目录 **MUST** 按路径段语义忽略；`.notegit-backup` 这类同名前缀兄弟路径 **MUST NOT** 被误判为内部目录。
- 被忽略 Markdown **MUST NOT** 通过 watcher/scan 生成 `Added`、`Modified`、`Deleted` 或 rename pending entry，也 **MUST NOT** 在 scan 中被当作 tracked doc 缺失处理。

### 8.4 Self-Write Suppression

- projection/persist_doc/commit apply 写盘前必须向 repo-local `WriteSuppressor` 注册写回指纹。
- watcher 在匹配窗口内必须丢弃自写事件。
- suppressor 状态必须 repo-local，禁止全局共享。

### 8.5 Overflow Recovery

- queue overflow / dropped events 时，watcher **MUST** 触发全量 reconcile。
- reconcile 完成前 **MUST** 暂停继续消费增量事件。

### 8.6 Lifecycle

- repo close / switch **MUST** 停止对应 watcher 并 drain 事件。
- 同一 repo **MUST NOT** 同时存在多个 watcher。

### 8.7 Debounce and Atomic Write Semantics

- debounce window **SHOULD** 为 `50ms-200ms`
- debounce window **MUST NOT** 为 `0`
- atomic write / temp-file replace 必须统一收敛成单次 pending modify / rename candidate
- rename pair 识别失败时，宁可退化为 pending delete + pending create，也不得伪造 authority rename

## 9. Recovery / Repair

### 9.1 Workspace Recovery

- Vault 损坏时，从 ledger + snapshot 重建 projection。
- 无法解释的 workspace 偏差视为状态漂移，必须 reconcile 或 hard rebuild。

### 9.2 Ledger Repair Boundary

- 只有显式 repair / reset 流程才允许从 Vault 反向导入生成新 ledger。
- 日常运行路径不得把 Vault 当成 authority fallback。

### 9.3 Catalog / Runtime Repair

- 允许修复 local/remote repo catalog、runtime tables、source control side tables。
- repair 不得伪造 authority history。

### 9.4 Backup / Export {#backup-export}

- repo **MAY** 定期生成只读 backup snapshot。
- 系统 **MUST** 支持将 ledger 导出为 JSON Lines。

### 9.5 Hard Failure vs Degraded Mode

- 以下情况允许进入 degraded mode：
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

## 10. Forbidden Patterns

- 原地修改 authority 状态。
- 用 metadata/path table 直接完成 rename/move/delete。
- 未经 Stage / Commit 让 watcher 事件直接入 ledger。
- 让 Vault 作为真值源。
- 让 side table 或 snapshot 成为删除真源。

## 11. Runtime Boundary

### 11.1 Authority Layer

- 负责 ledger append validation、runtime side table 归类、authority table 读写边界。
- 不得读取 UI 状态、watcher 原始事件或未归一化路径作为业务真相。

### 11.2 Projection / Workspace Layer

- 负责由 ledger fold 派生 projection、workspace writeback、projection cleanup 与 drift 解释。
- projection 失败不得伪装成 authority 成功。

### 11.3 Watcher Layer

- 负责外部文件事件归一化、忽略规则、debounce、self-write suppression 与 overflow reconcile。
- watcher 只能生成 pending candidate，不得直接写 ledger。

### 11.4 Repo Runtime Integration

- 负责 repo open/close、runtime directory bootstrap、catalog repair 与各层生命周期编排。
- 该层只能编排 authority/projection/watcher/repair，不得把 side table 升格为 authority。

## 12. Refactor Target

长期应显式形成四个 infra 子系统：

- `authority_storage_runtime`
- `projection_persistence_runtime`
- `watcher_runtime`
- `repair_runtime`

实现必须按这四层收敛；任何 manager/helper 只能作为其中一层的内部细节，不得跨层持有隐式 authority。

## 本章相关命令

- 无

## 本章相关配置

- `snapshot_depth`
- backup / retention 相关配置
- `vault.path`
- `ledger.path`

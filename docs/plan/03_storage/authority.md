# 03_storage/authority.md - Authority Storage Runtime

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-10`
- `Parent`: `03_storage/index`
- `Primary Code Areas`: `crates/core/src/ledger/`, `crates/core/src/ledger/manager/authority_storage_runtime.rs`, `crates/core/src/ledger/append_validate/`

> 本文件是 `03_storage` 章的 `authority_storage_runtime` 子合同：facts 分区、存储表与索引、write lifecycle 与 ledger-first 受控写路径。章节骨架与总览见 [index.md](./index.md)。

## 2. Authoritative Entities（authority 部分）

> §2.1 Core Stores 与 §2.2 Authority Model 见 [index.md](./index.md)。

### 2.3 Facts Partition {#facts-partition}

- `Content Facts`
  - 面向 `DocId`
  - Insert / Delete / replace 等文本变化
- `Structure Facts`
  - 面向 `NodeId / DocId`
  - Create / Rename / Move / Delete 等结构变化
- `Merge Anchor Facts`
  - 面向 `DocId + source PeerId`
  - 记录已确认 source waterline、local pre-merge waterline、resolution 与 result hash
  - 不直接改变内容/结构投影，但属于 ledger authority 并占用连续 `PeerFactSeq`

### 2.4 Non-Authoritative Runtime State

- `pending_fs_ops` (`PendingFsEntry` rows)
- `staging`
- `commit_index`
- `confirmed ledger dirty projection`
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

## 4. Storage Tables and Indexes

### 4.1 Core Tables

- `LEDGER_OPS: GlobalSeq -> LedgerEntry`
- `DOC_OPS: DocId -> [GlobalSeq]`
- `NODEID_TO_META: NodeId -> NodeMeta`
- `PATH_TO_NODEID: Path -> NodeId`
- `INODE_TO_NODEID: Inode -> NodeId`
- `SNAPSHOT_INDEX: DocId -> [SeqNo]`
- `SNAPSHOT_DATA: SeqNo -> ContentBlob`
- `PEER_FACT_SEQ: PeerId -> PeerFactSeq`
- `PEER_FACT_OPS: (PeerId, PeerFactSeq) -> GlobalSeq`
- `MERGE_BASE_CHECKPOINT: (SourcePeerId, DocId) -> MergeBaseCheckpoint`

### 4.1.1 Ledger Entry Format Contract {#ledger-entry-format-contract}

`LEDGER_OPS` 的 value 是 repo authority 的事实载荷，不得依赖 Rust struct 形状探测来判定版本。

规则：

- 每条 `LedgerEntry` 落盘 **MUST** 使用显式格式信封：magic header + `ledger_entry_format_version` + project-owned postcard codec payload。
- 当前格式为 `LEDGER_ENTRY_FORMAT_VERSION = 3`、magic `DEVELDG3`；v3 显式区分 `origin_peer_id`、`peer_seq` 与非权威诊断字段 `actor`。
- 读取路径 **MUST** 先验证 magic header，再按显式 `ledger_entry_format_version` dispatch。
- 运行时 **MUST NOT** 通过“尝试当前结构、再尝试若干 legacy 结构”的 codec 形状探测作为 authority decode 路径。
- 缺失 magic header、缺失版本或版本不受支持时，repo 必须 fail-closed；pre-1.0 未发布开发期旧 ledger 可要求显式 reset / repair / migration，不进入生产透明兼容承诺。

### 4.2 Sequence Contract

- `GlobalSeq`
  - repo 范围内全序
  - `LEDGER_OPS` 主键
- `PeerFactSeq`
  - repo 内 `(PeerId) -> PeerFactSeq`
  - 同一物理 peer 的全部 Content / Structure Facts 共享、从 1 开始且严格连续
- `PEER_FACT_OPS`
  - `(PeerId, PeerFactSeq) -> GlobalSeq`
  - 为 P2P peer range 提供唯一反向索引

规则：

- `GlobalSeq` 决定落盘顺序。
- `GlobalSeq` 不得进入 P2P VersionVector、wire envelope 或 source range。
- 本地事实的 `PeerFactSeq` 分配、`LEDGER_OPS` append、`PEER_FACT_SEQ` 水位与 `PEER_FACT_OPS` 反向索引必须在同一个 redb write transaction 提交；事务回滚不得留下永久序号缺口。
- `FactActor` 只用于诊断审计，不得参与序列、身份、去重或 source attribution；wire/storage decode 必须与构造器一致地拒绝空字符串和超过 64 bytes 的值，不能通过派生反序列化绕过类型不变量。
- 远端事实只能由认证 sync ingest 写入对应 source shadow；该路径验证既有 `(origin_peer_id, peer_seq)`，不得分配本地 peer sequence。
- `MergeAnchor` 必须由 host-bound local writer 追加；source peer、local/source waterline、source base hash 与 result hash 必须来自后端 merge preflight，前端不得提供或覆盖。
- `MERGE_BASE_CHECKPOINT` 只能与对应 `MergeAnchor` 在同一 local repo write transaction 更新；事务失败不得留下 anchor/checkpoint 任一侧的半提交。
- checkpoint 必须能反向定位到同一 `(source_peer_id, doc_id)` 的 anchor fact；悬空、hash 不一致或水位越界必须 fail-closed，repair 不得按内容相似度猜测共同祖先。

### 4.3 Runtime Side Tables and Repo Metadata

- `.notegit/` 或 repo runtime state 中至少要存在以下 side tables / metadata：
  - `pending_fs_ops`
    - key: stable pending op id
    - value: normalized path, op kind, inode/file id, content hash/base hash, detected_at
    - source: watcher / startup scan / directory rescan / explicit import
    - non-source: Web pending overlay
  - `staging`
    - key: staged entry id
    - value: pending source ref, staged_at, stage actor, repo head anchor, staged content hash
  - `commit_index`
    - key: commit id / anchor
    - value: ledger head, commit meta, affected docs/nodes
  - `projection_runtime`
    - value: last_projection_seq, last_projection_hash, degraded flag, repair marker
  - `watcher_runtime`
    - value: suppressor fingerprints, overflow marker, last_full_scan_at
- 这些 side tables **MUST** 明确标记为 workflow/runtime state，不得被上层误当成 authority state。
- `pending_fs_ops` 与 pending overlay **MUST** 分属不同状态域；二者不得复用同一 row、key space 或清理规则。
- confirmed ledger dirty projection **MUST** 只由 commit anchor 与 ledger head 派生；不得新增 side table 作为第二真源。

### 4.3.1 Redb Schema Version Gate {#redb-schema-version-contract}

每个 repo `.redb` **MUST** 在 `REPO_METADATA` 中携带顶层 schema version gate：

- `REPO_METADATA[0] = RepoInfo`
- `REPO_METADATA[1] = redb_schema_version`
- 当前 schema 为 `REDB_SCHEMA_VERSION = 3`，repo metadata 与 node metadata value 使用同一 project-owned postcard codec。

规则：

- 新建 local repo 与 remote shadow repo **MUST** 写入当前 `REDB_SCHEMA_VERSION`。
- 打开已有 repo 时，运行时 **MUST** 先校验 `REDB_SCHEMA_VERSION`，再读取 `RepoInfo` 或进入 ledger/query 路径。
- 缺失 schema version 或版本不匹配 **MUST** fail-closed，并暴露“需要显式迁移、reset 或重建”的诊断。
- v2 的 `peer_id/seq` 同时混入 actor 标签与 per-doc/per-node 计数，运行时不得推测性重写为 v3 物理 peer history。v2 只允许经 repair/export 兼容读取后显式导出，再重建 v3 repo。
- 表名后缀（如 `client_op_index_v2`）只能表达单个 side table 的内部演进，不得替代顶层 redb schema version gate。

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

## 5. State Machines（authority 部分）

> §5.1 Repo Mount Lifecycle 见 [index.md](./index.md)；§5.3 External Edit Lifecycle 见 [watcher.md](./watcher.md)。

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
  - effect: repo projection workspace matches projection or explicit dirty overlay
- `RecoverableProjectionFault`
  - effect: repo enters degraded-but-authority-valid state, waiting for rebuild

## 6. Ledger-First Write Paths（authority 部分）

> §6.2 Path B: Watcher / External Edit Ingestion 见 [watcher.md](./watcher.md)。

### 6.1 Path A: Controlled Editor / CLI Writes

1. 生成写入意图。
2. 校验 auth、repo binding、writer identity、append validity。
3. 生成 `Content Facts` / `Structure Facts`。
4. 追加到 ledger。
5. 重建或增量更新 projection。
6. 持久化回 workspace。
7. 该写入成为 `ConfirmedLedgerChange`，直到 Source Control commit anchor 覆盖当前 ledger head。

规则：

- **MUST NOT** 先改 Projection Workspace 再补 ledger。
- Path A 写入成功后 **MUST NOT** 回灌到 `pending_fs_ops` 或 staging；Source Control 只能通过 confirmed ledger dirty projection 展示它。

### 6.3 Path C: Stage -> Apply to Ledger -> Commit Anchor

1. 从 `pending_fs_ops` 迁移到 `staging`；一个用户 stage batch 的 pending remove、staged insert 与两侧 DocId index 更新必须在同一事务提交。
2. staging 保存检测时的内容 hash；Apply preflight 重新读取 workspace 并验证非 delete 文件仍与该 hash 一致，同时捕获本次 Apply 唯一允许消费的内容快照。
3. 以当前 confirmed projection 为 base 计算差异。
4. `Apply to Ledger` 生成 content / structure facts 并追加到 ledger。
5. Apply 成功后清理 External Changes staging 并回写 projection；此时变化进入 confirmed ledger dirty。
6. 后续 Source Control commit 只为 confirmed ledger dirty 写 commit anchor。

额外约束：

- stage 是真实迁移，不是 UI 布尔标记。
- Apply 生成 diff 时 base **MUST** 是当前 confirmed projection，而不是当前 workspace 内容快照。
- staging 后 workspace 内容发生变化时，Apply **MUST** fail-closed、保留 staging，并要求重新 scan/stage；不得静默应用未确认的新内容。
- Apply **MUST NOT** 在 hash preflight 后再次从 workspace 读取内容；本批所有 structure/content facts、identity index 更新与本次 staged snapshot 的 exact consumption 必须在同一 ledger write transaction 提交，任一 target 失败则整批回滚。事务开始时必须比较 preflight 前捕获的 ledger head；head 漂移表示 confirmed/content base 已变化，整批 fail-closed 并要求刷新重试。事务不得清空 preflight 后新加入的其他 staging；原 staged row 被替换或移除时必须 fail-closed。
- discard 的语义只能是“恢复 vault 到 projection + 清理 pending/staging”，不得触碰 ledger history。
- 当 staged 为空但存在 `ConfirmedLedgerChange` 时，commit **MUST** 只创建覆盖当前 ledger head 的 commit anchor，不得重复追加内容或结构 facts。
- ordinary External Changes staging **MUST NOT** 被普通 commit 消费；只有显式 resolved-conflict staging 可以按 `05_diff_logic` 的受控例外在同一 writer gate 内 apply 后创建 anchor。

## 10. Forbidden Patterns（authority）

> 跨层禁止项见 [index.md](./index.md)。

- 用 metadata/path table 直接完成 rename/move/delete。
- 未经 Stage / Commit 让 watcher 事件直接入 ledger。

## 11. Runtime Boundary（authority 部分）

### 11.1 Authority Layer

- 负责 ledger append validation、runtime side table 归类、authority table 读写边界。
- 不得读取 UI 状态、watcher 原始事件或未归一化路径作为业务真相。

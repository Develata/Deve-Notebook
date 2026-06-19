# 03_storage/authority.md - Authority Storage Runtime

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-06-20`
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

### 2.4 Non-Authoritative Runtime State

- `pending_fs_ops` (`PendingFsEntry` rows)
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

## 4. Storage Tables and Indexes

### 4.1 Core Tables

- `LEDGER_OPS: GlobalSeq -> LedgerEntry`
- `DOC_OPS: DocId -> [GlobalSeq]`
- `NODEID_TO_META: NodeId -> NodeMeta`
- `PATH_TO_NODEID: Path -> NodeId`
- `INODE_TO_NODEID: Inode -> NodeId`
- `SNAPSHOT_INDEX: DocId -> [SeqNo]`
- `SNAPSHOT_DATA: SeqNo -> ContentBlob`

### 4.1.1 Ledger Entry Format Contract {#ledger-entry-format-contract}

`LEDGER_OPS` 的 value 是 repo authority 的事实载荷，不得依赖 Rust struct 形状探测来判定版本。

规则：

- 每条 `LedgerEntry` 落盘 **MUST** 使用显式格式信封：固定 magic header + `ledger_entry_format_version` + bincode payload。
- 当前首版格式为 `LEDGER_ENTRY_FORMAT_VERSION = 1`。
- 读取路径 **MUST** 先验证 magic header，再按显式 `ledger_entry_format_version` dispatch。
- 运行时 **MUST NOT** 通过“尝试当前结构、再尝试若干 legacy 结构”的 bincode 形状探测作为 authority decode 路径。
- 缺失 magic header、缺失版本或版本不受支持时，repo 必须 fail-closed；pre-1.0 未发布开发期旧 ledger 可要求显式 reset / repair / migration，不进入生产透明兼容承诺。

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
    - source: watcher / startup scan / directory rescan / explicit import
    - non-source: Web pending overlay
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
- `pending_fs_ops` 与 pending overlay **MUST** 分属不同状态域；二者不得复用同一 row、key space 或清理规则。

### 4.3.1 Redb Schema Version Gate {#redb-schema-version-contract}

每个 repo `.redb` **MUST** 在 `REPO_METADATA` 中携带顶层 schema version gate：

- `REPO_METADATA[0] = RepoInfo`
- `REPO_METADATA[1] = redb_schema_version`
- 当前首版 schema 为 `REDB_SCHEMA_VERSION = 1`。

规则：

- 新建 local repo 与 remote shadow repo **MUST** 写入当前 `REDB_SCHEMA_VERSION`。
- 打开已有 repo 时，运行时 **MUST** 先校验 `REDB_SCHEMA_VERSION`，再读取 `RepoInfo` 或进入 ledger/query 路径。
- 缺失 schema version 或版本不匹配 **MUST** fail-closed，并暴露“需要显式迁移、reset 或重建”的诊断。
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

规则：

- **MUST NOT** 先改 Projection Workspace 再补 ledger。

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

## 10. Forbidden Patterns（authority）

> 跨层禁止项见 [index.md](./index.md)。

- 用 metadata/path table 直接完成 rename/move/delete。
- 未经 Stage / Commit 让 watcher 事件直接入 ledger。

## 11. Runtime Boundary（authority 部分）

### 11.1 Authority Layer

- 负责 ledger append validation、runtime side table 归类、authority table 读写边界。
- 不得读取 UI 状态、watcher 原始事件或未归一化路径作为业务真相。

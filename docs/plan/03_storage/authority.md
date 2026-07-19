# 03_storage/authority.md - Authority Storage Runtime

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-19`
- `Parent`: `03_storage/index`
- `Primary Code Areas`: `crates/core/src/ledger/`, `crates/core/src/ledger/manager/authority_storage_runtime.rs`, `crates/core/src/ledger/append_validate/`, `crates/core/src/remote_import/`

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
  - `projection_faults`
    - key: project-owned fault id
    - value: versioned repo-local recovery evidence、typed origin、Ledger head/range 与 retry state
  - `watcher_runtime`
    - value: suppressor fingerprints, overflow marker, last_full_scan_at
- 这些 side tables **MUST** 明确标记为 workflow/runtime state，不得被上层误当成 authority state。
- `pending_fs_ops` 与 pending overlay **MUST** 分属不同状态域；二者不得复用同一 row、key space 或清理规则。
- confirmed ledger dirty projection **MUST** 只由 commit anchor 与 ledger head 派生；不得新增 side table 作为第二真源。

### 4.3.1 Redb Schema Version Gate {#redb-schema-version-contract}

每个 repo `.redb` **MUST** 在 `REPO_METADATA` 中携带顶层 schema version gate：

- `REPO_METADATA[0] = RepoInfo`
- `REPO_METADATA[1] = redb_schema_version`
- 首个正式 tag 目标 schema 为 `REDB_SCHEMA_VERSION = 4`，repo metadata、node metadata 与 Remote Import workflow value 使用同一 project-owned postcard codec。

规则：

- 新建 local repo 与 remote shadow repo **MUST** 写入当前 `REDB_SCHEMA_VERSION`。
- 打开已有 repo 时，运行时 **MUST** 先校验 `REDB_SCHEMA_VERSION`，再读取 `RepoInfo` 或进入 ledger/query 路径。
- 缺失 schema version 或版本不匹配 **MUST** fail-closed，并暴露“需要显式迁移、reset 或重建”的诊断。
- v2 的 `peer_id/seq` 同时混入 actor 标签与 per-doc/per-node 计数，运行时不得推测性重写为当前物理 peer history。v2 只允许经 repair/export 兼容读取后显式导出，再重建当前 repo。
- 未发布的 v3 database 不做原地 v4 migration、adapter 或双轨读取。B1 切换前代码仍是 v3；B1 必须一次性把新建/打开 gate 切到 v4，旧开发数据只能在旧 HEAD 显式导出后重建。
- 表名后缀（如 `client_op_index_v2`）只能表达单个 side table 的内部演进，不得替代顶层 redb schema version gate。

### 4.3.2 Remote Import Workflow Tables {#remote-import-workflow-tables}

Redb v4 新增两个 project-owned workflow table：

```rust
REMOTE_IMPORT_SESSIONS: TableDefinition<u128, &[u8]>
REMOTE_IMPORT_RUNTIME: TableDefinition<u8, &[u8]>
```

- `REMOTE_IMPORT_SESSIONS` 保存 session identity、repo/branch/head binding、状态、
  manifest/candidate/blob aggregate digests、terminal receipt 与
  `cleanup_pending`；不内嵌 provider payload 或 workspace content。
- `REMOTE_IMPORT_RUNTIME` 保存每 repo active pointer、CAS generation 与
  schema-owned recovery metadata。每 repo 最多一个 active session；不存在
  durable `Applying`。Apply 的 single-flight 只存在于当前 process runtime，
  durable 并发与重试结果由 Redb CAS 和 stored receipt 裁决。
- `Preparing` 启动恢复必须变成 `Failed(Interrupted)`；`Ready/Stale/Failed`
  与 terminal `Applied/Discarded` 的完整状态机归
  `06_backup#remote-import-state-machine`。
- terminal record 最近保留 64 个；`cleanup_pending=true` 永不自动裁剪。
- filesystem manifest/blob names、mtime、目录存在性或 provider metadata不得
  反推 Redb session state。digest mismatch、orphan 或 incomplete publication
  只能进入 typed Failed/repair。

### 4.3.3 Projection Fault Recovery Table {#projection-fault-recovery-table}

Redb v4 local-authority profile 还必须包含
`PROJECTION_FAULTS: TableDefinition<[u8; 32], &[u8]>`。该表不是 Remote Import
workflow table，也不是 Ledger authority；它只保存 repo-local、host-only 的
Projection recovery evidence，唯一 mutation contract 归
`03_storage/projection#durable-projection-fault-contract`。Remote shadow 不创建或消费该表。

### 4.4 Snapshot Storage Contract

- snapshot 采用 dual-table：
  - `SNAPSHOT_INDEX`
    - `DocId -> [SeqNo]`
  - `SNAPSHOT_DATA`
    - `SeqNo -> ContentBlob`
- 规则：
  - snapshot 永远锚定到已确认 `GlobalSeq`
  - 保存 snapshot 前必须从 Ledger 全量重建该 `DocId` 在目标 `GlobalSeq` 的内容，并与候选内容逐字节严格相等；不得以长度、头尾采样或 hash sampling 代替全量一致性证明
  - 候选内容与全量重建结果不一致时必须拒绝保存，既有 dual-table snapshot 状态保持不变
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

### 6.1.1 Repository Mutation Publication Gate {#repo-mutation-publication-gate}

所有在线、进程内、面向 Local Branch 的 authority writer **MUST** 经过同一个
process-scoped `RepoMutationPublicationGate`。该 gate 以 `RepoId` 为粒度串行化同一 repo
的本地写入，不同 repo 仍可并行；registry 必须使用弱引用回收闲置 repo permit，不得让
访问过的 repo 永久累积。

所有依赖 Projection Workspace 当前性的在线本地操作还 **MUST** 进入统一 typed
`execute_mounted_repo`。该执行面组合 `RepoMutationPublicationGate` 与只读
`WatcherRuntimeView`，并按固定顺序完成准入：

```text
repo scope / branch
  -> projection / locator health
  -> acquire Mounted(repo_id, generation) admission token
  -> repo mutation permit
  -> exact revalidation: same slot + same generation + Mounted
  -> authority/workspace side effects
```

`WatcherRuntimeView` 的初检必须返回只读 `MountAdmissionToken`，绑定稳定 slot identity、
`RepoId` 与 generation；初检只短时读取 supervisor map，不得持锁等待 repo permit。
取得 repo permit 后、任何副作用前，gate 必须通过 token 对同一原子 slot state 做 exact
revalidation。该 revalidation 是 mutation admission cut；token 指向的旧 slot 在移除或替换前
必须先原子退出 Mounted，因此不得通过悬空 token 绕过新 generation。

mount revalidation 与 watcher `Failed/Transitioning` 状态转换必须在同一个 slot 原子状态上
线性化。failure cut 之前完成 exact revalidation 的 operation 可以完成；即使请求在 transition
前已通过初检，只要排队取得 permit 后落在 failure/transition cut 之后，也必须在任何
workspace/ledger/staging/Git side effect 之前返回
`STORAGE_WORKSPACE_INGESTION_UNAVAILABLE`。该错误码与 HTTP/WS 映射由
`13_i18n#i18n-error-code-catalog` 和 `07_network` 唯一定义。

已越过 admission cut、但按合同必须在 permit 外执行 scan 的 operation，可以持有一次性
completion continuation。continuation 必须同时绑定原 mount slot/generation 与 repo-lane
revision：repo-local watcher 在 cut 后进入 `Failed` 不得阻断该 operation 的清理/最终化；
slot replacement、lifecycle transition 或 scan 期间任何后续受管 repo writer 则必须使旧
continuation fail-closed。旧 continuation 不得据此执行可能覆盖新 writer 的 rollback；应保留
可诊断 staging/workspace 事实并进入显式 repair。

supervisor map mutex 不得跨 repo permit 获取或等待、I/O、scan、join、await 与 publication；
持有 map mutex 时禁止获取 Catalog/Repo permit。exact revalidation 只读 token 指向的原子
slot state，不重新获取 supervisor map mutex，从而避免 Map↔Repo 反向嵌套。

`execute_mounted_repo` 的覆盖范围包括：

- editor append/writeback；
- Docs create/copy/rename/move/delete；
- External Changes prepare/stage/unstage/discard/apply；
- Source Control commit/push 中读取或改变 local workspace/authority 的部分；
- local merge、projection apply 与 Remote Import Apply；
- plugin note writer 与 source-control writer。

纯读、ledger inspect/export、认证 remote-shadow ingest、offline repair/export/diagnostic 不受
mount gate 阻断。它们仍必须遵守各自的 scope、authority 与安全合同，不得借此进入在线
workspace mutation。Watcher 自身只写 External Changes pending，不进入 authority mutation
permit；其 pending/reconcile 写入由 owned watcher generation 与 lifecycle contract 串行化。

Repo catalog create 使用独立的 typed `Catalog -> Repo(new RepoId)` lane；RemoveLocalRepo 的短
authority cut 必须按固定 `Catalog -> Repo(RepoId)` 顺序同时持有 catalog 与目标 repo permit。
两者的唯一 durable normal-membership linearization fact 是
`ledger/.host/repo-catalog/<repo_id>.json` 的单记录原子发布/替换；cut 内只允许对该单记录执行
bounded exact read + temp/flush/replace/directory-sync，不允许其它 filesystem I/O、scan、目录遍历、
watcher/provider I/O、session fan-out 或 await。DB、locator、workspace marker 的 revalidation token
必须在 permits 外形成，且不能单独授权正常 listing 或 writer admission。
Host-local alias set/import 不是 repo authority mutation，不获取 watcher lifecycle reservation，
只进入 alias runtime 的短 CAS。不得用 nil UUID 或字符串
哨兵伪装 catalog identity，也不得允许 repo lifecycle 与该 repo 的 authority writer 并发。已经持有任意
repo permit 的调用不得再获取 `Catalog` lane；这一反向嵌套必须 fail-closed，避免与
`Catalog -> Repo(RepoId)` 形成 ABBA 死锁。

authority mutation permit 的覆盖范围包括 Browser Edit、Docs create/copy/rename/move/delete 与 repo authority mutation、
External Apply、Source Control commit（含 resolved-conflict 与 commit-and-push 的 authority
部分）、Remote Import Apply、merge result、plugin `note_write`，以及未来启用的 local reconcile。Watcher 只写
External Changes pending，不进入该 gate；认证 remote shadow ingest、离线 repair/export 与
diagnostic 明确排除。Remote Import Prepare/Show/Page/Diff/Refresh/Discard 走 session runtime
自己的 repo/session CAS；它们不获得 Ledger writer authority，也不要求 Mounted。

repo selector、alias 或路径只用于锁外定位候选身份。每个 writer 在获得 permit 后 **MUST** 重新解析并
exact-compare 当前 `RepoId`、CatalogMembershipToken 与 local-writable 状态；remove 后同 alias 新 repo 不得继承旧请求
的写权限。Git mirror queue 等延后投影操作必须携带提交时的 expected `RepoId`，执行时再次验证，不能只按
可复用的 repo 名称回查。

固定锁序为：

```text
local repo mutation permit -> shadow merge guard -> redb write transaction
```

同一调用不得嵌套获取相同 repo permit。目录扫描、正文重建、diff/patch 计算、External Apply preflight、
resolved-conflict commit preflight 与 Rhai 执行必须在 permit 外完成；获得 permit 后必须 exact-compare
preflight 捕获的 repo identity、ledger head、path/staging evidence。临界区只允许 mutable revalidation、authority
transaction、必要 projection writeback 或 degraded marker、typed receipt 构造与 publication
enqueue；目录扫描、diff、正文预读、网络、Rhai、HTTP response 与 Git mirror 必须在锁外。

server runtime 必须用 typed `MutationExecution` 区分：

- `NotCommitted`：authority 未提交，不发布成功或恢复信号。
- `Committed`：完整提交，并在释放 permit 前按顺序 enqueue 对应确认或恢复消息。
- `ProjectionDegraded`：authority 已提交但 projection/writeback 降级；仍发布恢复消息并报告 degraded。
- `CommittedPartial`：仅用于仍由多个旧事务组成、无法伪称原子的流程；只要已有 authority
  effect 就必须发布恢复消息并报告 partial。

普通单次编辑仍发布一个 confirmed `NewOp`。External Apply、Docs bulk、merge、plugin 等
批量/跨投影写入只发布一个 typed projection recovery；不得把已提交后的 enrichment 或
projection 查询失败伪报为“authority 未提交”。

### 6.3 Path C: Stage -> Apply to Ledger -> Commit Anchor

1. 从 `pending_fs_ops` 迁移到 `staging`；一个用户 stage batch 的 pending remove、staged insert 与两侧 DocId index 更新必须在同一事务提交。
2. staging 保存检测时的内容 hash；Apply preflight 重新读取 workspace 并验证非 delete 文件仍与该 hash 一致，同时捕获本次 Apply 唯一允许消费的内容快照。
3. 以当前 confirmed projection 为 base 计算差异。
4. `Apply to Ledger` 生成 content / structure facts 并追加到 ledger。
5. Apply 成功后清理 External Changes staging 并回写 projection；此时变化进入 confirmed ledger dirty。
6. 后续 Source Control commit 只为 confirmed ledger dirty 写 commit anchor。

额外约束：

- stage 是真实迁移，不是 UI 布尔标记。
- unstage 必须在一个 redb write transaction 内重新解析目标并 exact-compare staged row；staged remove、pending upsert 与两侧 `DocId` index 更新必须共同提交。若同路径已有语义不同的较新 pending 证据则整笔 fail-closed，不得覆盖；任一步失败都保留原 staged/pending 状态。
- Apply 生成 diff 时 base **MUST** 是当前 confirmed projection，而不是当前 workspace 内容快照。
- staging 后 workspace 内容发生变化时，Apply **MUST** fail-closed、保留 staging，并要求重新 scan/stage；不得静默应用未确认的新内容。
- Apply **MUST NOT** 在 hash preflight 后再次从 workspace 读取内容；本批所有 structure/content facts、identity index 更新与本次 staged snapshot 的 exact consumption 必须在同一 ledger write transaction 提交，任一 target 失败则整批回滚。事务开始时必须比较 preflight 前捕获的 ledger head；head 漂移表示 confirmed/content base 已变化，整批 fail-closed 并要求刷新重试。事务不得清空 preflight 后新加入的其他 staging；原 staged row 被替换或移除时必须 fail-closed。
- Apply 成功必须返回 `ExternalApplyReceipt { repo_id, authority_head: GlobalSeq, affected_docs, applied_target_count }`。receipt 只描述已经提交的 authority 结果；不得在 commit 后依赖 confirmed-list enrichment 才判定成功，也不得携带由 Web 重算的逐 fact diff。
- discard 的语义只能是“恢复 vault 到 projection + 清理 pending/staging”，不得触碰 ledger history。
- 当 staged 为空但存在 `ConfirmedLedgerChange` 时，commit **MUST** 只创建覆盖当前 ledger head 的 commit anchor，不得重复追加内容或结构 facts。
- commit 覆盖 confirmed ledger dirty 时，全部 committed snapshot baselines 与对应 commit payload/order anchor 必须在同一个 redb write transaction 提交；任一 snapshot 或 anchor/order 写入失败都不得留下半提交。Git mirror queue 只能在该事务成功后作为可恢复 projection 操作排队，排队失败不得回滚 NoteGit commit。
- ordinary External Changes staging **MUST NOT** 被普通 commit 消费；只有显式 resolved-conflict staging 可以按 `05_diff_logic` 的受控例外在同一 writer gate 内 apply 后创建 anchor。

### 6.3.1 Sealed Prepared Ledger Change Batch {#sealed-ledger-change-batch}

`PreparedLedgerChangeBatch` 是 crate-private、不可复制 authority capability。
只有 External Apply 与 Remote Import 的 source-specific constructor 能构造；
不得暴露 generic callback、public fact vector constructor、半事务 append 或让
session/provider runtime直接操作 Ledger authority tables。

Remote Import constructor 必须在进入同一个 Redb write transaction 前准备好
全部 immutable inputs，并在事务内精确复核：

- `RepoId`、schema v4、current ledger head 与 active session pointer；
- session id、candidate revision、manifest digest、candidate digest 与全部 blob digest；
- writer identity、local branch、Projection Locator 与 `.deveignore` snapshot；
- pending/staged overlap，以及 session 仍属于当前 catalog `RepoId`。

同一事务必须完成全部 upsert Content/Structure Facts、identity/index 更新、
state=`Applied`、带 immutable authority commit core 且
projection outcome=`Pending` 的 `RemoteImportApplyReceipt`、active pointer clear 与
`cleanup_pending=true`。任一复核或写入失败整笔回滚，不得留下事实前缀。

`Pending` 不是 durable `Applying`，也不允许第二次 append；它只表示 Ledger 已提交而 post-commit
projection outcome 尚未持久化。事务提交后，projection runtime 从 Ledger facts 幂等 writeback：

- writeback 成功后，以第二个短 Redb transaction 把 receipt outcome 从 `Pending` CAS 为 `Written`；
- writeback 失败时，由 Remote Import post-commit coordinator 开启第二个短 Redb
  transaction，通过 projection runtime 的窄化 typed API 写入 repo-local
  `PROJECTION_FAULTS` evidence，并由 Remote Import 自有 store API 把 outcome 从
  `Pending` CAS 为 `Degraded`；
- 进程在任一时点退出或第二个 transaction 失败时，receipt 保持 `Pending`。启动恢复或相同
  request 重试必须识别 immutable authority core，重放幂等 projection writeback 并继续上述 CAS，
  绝不能重新 append Ledger facts。若 filesystem writeback 已成功但 outcome 尚未更新，重复
  materialization 也必须安全。

第二个 transaction 只更新 workflow receipt outcome 与 repo-local `PROJECTION_FAULTS` side table，不包含或补写
任何 Content/Structure Fact、identity/index 或 active-session authority；因此 whole-session Ledger
transaction 仍是唯一且不可拆分的事实提交边界。

response 只能报告已 durable 的 receipt 当前态：`Written` 返回正常 Applied；`Degraded` 返回
“Ledger 已提交、Projection degraded”；`Pending` 必须明确返回“Ledger 已提交、Projection recovery
pending”的 typed outcome，不能伪装未提交。相同 session/revision/request 在响应丢失后返回并收敛
已存 receipt，不得创建新的事实批次或回滚 Ledger。

## 10. Forbidden Patterns（authority）

> 跨层禁止项见 [index.md](./index.md)。

- 用 metadata/path table 直接完成 rename/move/delete。
- 未经 Stage / Commit 让 watcher 事件直接入 ledger。

## 11. Runtime Boundary（authority 部分）

### 11.1 Authority Layer

- 负责 ledger append validation、runtime side table 归类、authority table 读写边界。
- 不得读取 UI 状态、watcher 原始事件或未归一化路径作为业务真相。

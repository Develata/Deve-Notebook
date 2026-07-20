# 03_storage/projection.md - Projection & Persistence Runtime

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-19`
- `Parent`: `03_storage/index`
- `Primary Code Areas`: `crates/core/src/sync/projection_persistence_runtime.rs`, `crates/core/src/projection_fault/`, `crates/core/src/ledger/manager/projection_locator.rs`, `crates/core/src/sync/projection_io.rs`, `crates/core/src/writeback/`

> 本文件是 `03_storage` 章的 `projection_persistence_runtime` 子合同：projection locator layout 与 projection / persistence contract。`projection_locator_runtime` 在该顶层 storage infra 内作为独立命名的 host-local 子 runtime，拥有 locator 状态与文件边界，但不增加第五个顶层 storage runtime。章节骨架与总览见 [index.md](./index.md)。

## 3. Physical Layout（projection 部分）

> §3.2 Repo Runtime Layout 见 [index.md#repo-runtime-layout](./index.md#repo-runtime-layout)。

### 3.2.1 Projection Locator Layout {#projection-locator-contract}

Projection Locator 是 host-local runtime state，负责把本地 repo instance 绑定到宿主文件系统中的 projection base 与不可变物理 workspace segment。最终 repo workspace root 只由 locator 计算：

```text
ProjectionWorkspaceRoot(repo_id) = projection_base_abs / workspace_segment
```

示例：

```text
<projection_base>/<workspace_segment>/
  .notegit/
  .deveignore
  a.md
  notes/a.md
```

`projection_base` 可以包含其它文件或目录；系统只能 scan/watch/import locator 绑定的 `<projection_base>/<workspace_segment>/`，不得把 base 根目录本身当作 repo workspace。

路径归属判定示例：

- 本机新建 repo 时，可以把创建时 alias 规范化为 `my-notebooks--550e8400-e29b-41d4-a716-446655440000` 并一次写入 `workspace_segment`；之后 alias 变化不得移动该目录。
- 从其它 peer 首次发现且本机未配置 alias 时，`workspace_segment` 使用 canonical `550e8400-e29b-41d4-a716-446655440000`；远端不得提供本机默认 alias 或路径段。

最小模型：

```text
ProjectionLocatorKey = RepoId
ProjectionLocatorValue = {
  repo_id,
  projection_base_abs,
  workspace_segment,
  canonicalized_at,
}
```

Runtime ownership：

- `projection_locator_runtime` 是 `projection_persistence_runtime` 下的独立子 runtime，唯一拥有
  `ledger/.host/projection-locators.toml` 的读取、校验与变更权限。
- projection persistence、repo scope 与 repair 只能通过 locator 的 typed query / command
  使用或变更 locator；不得各自解析或写入 locator 文件。
- locator runtime 只拥有 host-local `RepoId -> (projection_base, workspace_segment)` 绑定、workspace root 派生、
  admission 与冲突检查；它不拥有 ledger facts、projection 内容、workspace writeback、
  repo identity 或 repair 状态迁移。
- locator runtime 必须提供独立的 typed `prepare repo creation locator` command。该 command 只允许为一个尚未进入
  normal catalog membership、但 canonical `<repo_id>.redb` metadata 已精确证明同一 `RepoId` 的
  `PreparedRepoCreation` 写入 locator；它必须同时按正常规则校验其余 cataloged locator、完整路径冲突与
  workspace containment，不得把 prepared locator 暴露给正常 query/list/admission。普通 locator set/query
  继续要求 normal catalog membership，禁止用通用 `allow unknown` 参数绕过该边界。

约束：

- `projection_base_abs` **MUST** 是 canonicalize 后的绝对路径；若 base 不存在，`init` / locator repair 可以先创建 base，再 canonicalize。
- `workspace_segment` 在 locator 创建时必须经过单一路径段校验：不得包含路径分隔符、drive prefix、NUL、Windows 非法字符（`< > : " | ? *`），不得等于 `.` / `..`，不得使用 Windows reserved device name（大小写不敏感），不得以空格或点结尾，并且必须经过同目录冲突检查。
- prepared locator command 只能新增或 exact-replace 当前 creation target 的记录；target DB identity、locator
  `repo_id` 与 workspace marker 必须在 catalog cut 前再次 revalidate。cut 前失败只能按 prepared manifest
  精确清理该 target，不能影响其它 locator；cut 成功后该记录才可由正常 locator query/admission 使用。
- 本机 create 可使用 `<safe_initial_alias>--<full_repo_id>`；其它首次绑定默认使用 `<full_repo_id>`。实现不得使用短 id 作为唯一判定依据。
- locator 一旦提交，`workspace_segment` 在该 RepoId 的本机生命周期内永久不变。普通 alias set/import **MUST NOT** 改写它；显式 workspace relocation 只能替换 `projection_base_abs` 并复用同一 segment，必须独立执行 watcher stop、identity admission、locator binding generation rotation 与 rematerialize，不得伪装成 alias rename。
- 本地可写 repo 进入 `ProjectionReady` 前 **MUST** 存在 locator。
- locator **MUST NOT** 写入 `LEDGER_OPS`、Structure Facts、Content Facts 或 sync payload。
- locator **MUST NOT** 作为 repo identity；`workspace_segment` 只能定位当前 host 的物理 projection，不得替代 `RepoId`。
- workspace root 是派生值。实现可以缓存 `workspace_root_abs`，但缓存 **MUST** 可由 `projection_base_abs + workspace_segment` 重建，且不得成为独立 authority。
- workspace admission **MUST** 校验 `.notegit` identity marker 中的 `repo_id` 等于当前 `RepoId`；路径名匹配但 marker 缺失或不一致时必须 fail-closed。
- host-local alias 修改不得触发 workspace realign、watcher lifecycle、Projection Fault 或 Remote Import stale。
- 两个本地 repo **MUST NOT** 解析到同一 workspace root。
- 任意两个 workspace root **MUST NOT** 互为父子目录。
- workspace root **MUST NOT** 位于 `ledger/`、`ledger/.host/`、`.notegit/` 或 `.git/` 内部。
- locator 缺失、路径不可读、路径不可 canonicalize 或路径冲突时，repo **MUST** 进入 `DegradedLocator`，不得进入 mounted write path。

## 7. Projection and Persistence Contract {#projection-contract}

- 所有系统写盘都必须满足：

```text
Intent -> Ledger Facts -> Projection -> Projection Workspace
```

- `metadata`、`path mapping`、`tree cache`、`NodeMeta` 只能由 projection builder 写入。
- handler、component、source control action 不得把这些表当成主写路径。
- ledger append 成功而 projection 失败时，系统 **MUST** 标记 recoverable fault，并支持从 ledger 重建；对 writeback / realign / rebuild interrupted 这类重启后仍需精确重放的故障，唯一 durable mutation contract 见本章 §7.2，观测到 RepoHealth 的映射见 `22_reliability_observability.md#observation-to-health-mapping`。

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
- projection writeback **MUST** 通过 Projection Locator 解析 `projection_base + workspace_segment` 作为目标 workspace root；禁止从全局 vault root或当前 alias 隐式推断。
- repo child path **MUST** 是 canonical forward-slash relative path。root lookup 可以用空 `repo_path` 返回计算出的 workspace root；任意非空 child path 必须在单一 core locator 入口拒绝 absolute path、任意 segment 中的 drive / UNC prefix、NUL、空 segment（包括 `//` 与首尾 `/`）、`.`、`..`、反斜杠、Windows 会折叠的尾随点或空格，以及任意层级大小写不敏感的 `.git` / `.notegit` segment；调用方不得各自重复或放宽该校验。
- 若计算出的 workspace root 已存在，locator 入口 **MUST** 拒绝 workspace root 自身为 symlink / junction，canonicalize 后的 root 必须仍是 canonical projection base 的直接子目录；随后从 joined target 自身向上找到最近的 existing ancestor，该 ancestor 必须可 canonicalize 且 canonical path 位于 canonical workspace root 内。existing child symlink / junction 指向 workspace root 外部、dangling symlink、stat / canonicalize 失败都必须 fail-closed，不得继续 write / remove / materialize。
- 若 workspace root 尚不存在，locator 入口只能返回通过上述 lexical gate 的 safe join；后续创建 root 的调用方不得由 child path 创建或越出 `<projection_base>/<workspace_segment>/`。
- 这些 path / containment gate 只约束 Projection Workspace 派生路径，不改变 Ledger authority、storage schema、RepoId 或 repo identity。

### 7.1 Remote Import Projection Writeback {#remote-import-projection-writeback}

- Remote Import Prepare/Show/Page/Diff/Refresh/Discard **MUST NOT** write, stage, scan,
  or pre-project provider bytes into the Projection Workspace or External
  Changes. Sealed manifests/blobs remain under the host-only layout in
  `03_storage/index#remote-import-runtime-layout`.
- Apply enters the same ordered projection persistence boundary only after its
  whole-session Ledger transaction commits. The builder reads the newly
  committed facts; it must not trust provider paths, candidate display labels,
  or browser diff state as writeback input.
- The authority transaction first stores an Applied receipt with immutable
  commit core and projection outcome `Pending`. Writeback success monotonically
  CASes it to `Written`; writeback failure atomically stores the normal durable
  projection fault evidence and CASes it to `Degraded`. Crash/retry while
  `Pending` rematerializes idempotently from Ledger and must never append facts.
- Writeback failure cannot reverse an Applied session or Ledger facts. The
  returned typed outcome must distinguish committed/pending recovery from
  committed/projection-degraded and remain recoverable by Ledger rematerialization.
- Successful writeback uses the existing locator, containment, PersistGuard,
  watcher-suppression, and repo-scoped ordering rules; Remote Import does not
  create a second projection writer.

### 7.2 Durable Projection Fault Store {#durable-projection-fault-contract}

`projection_persistence_runtime` 唯一拥有 repo-local Redb v4
`PROJECTION_FAULTS: TableDefinition<[u8; 32], &[u8]>` 的 typed query / mutation API。
该表记录 host-local recovery evidence；它不是 Ledger Fact、不同步、不参与
`GlobalSeq` / `PeerFactSeq`、不改变 Projection format，也不成为 RepoHealth 或
workspace 的第二真源。

每条 project-owned postcard value 必须携带显式 value version，并至少包含：

```text
DurableProjectionFault = {
  repo_id,
  fault_kind,
  typed_origin,
  target_path?,
  source_path?,
  doc_id?,
  ledger_seq_or_head?,
  first_seen_at,
  last_seen_at,
  last_error_bounded,
  retry_count,
  status=Pending,
}
```

alias 与任何历史 display label 都不参与 fault identity、codec 或 replay admission；人工诊断需要显示名时，
由当前 host alias runtime 在查询时另行投影，不能冻结进 durable fault value。

`deterministic_fault_key` 是 Redb table key，不冗余写入 postcard value；读取时必须由
value 的 identity fields 重算并与 table key exact-compare。

`typed_origin` 至少区分普通 projection persistence、projection repair 与
`RemoteImport { session_id, revision, request_id }`。路径只作诊断/精确重放输入，必须
forward-slash 规范化；repair 仍须按当前 `RepoId`、locator 与 workspace identity marker
重新 admission，不能信任记录中的 display alias 或旧路径。

key 必须使用 project-owned domain-separated SHA-256 构造，输入为 `RepoId + fault_kind +
typed_origin + normalized semantic target`，字符串使用显式长度前缀、整数使用固定大端编码；
不得直接 hash Rust enum/postcard bytes。读取时必须从 value 重算 key 并 exact-compare。
unsupported value version、RepoId mismatch、malformed/trailing payload 或 key/value mismatch
必须 fail-closed。

Mutation rules：

- ordinary writeback/rebuild failure 必须在 transaction 外完成 locator/path normalization、
  timestamp 与 bounded diagnostic 准备，再通过短 repo-local Redb transaction 幂等 upsert；同一
  typed origin / kind / target identity 的重复故障只更新 last-seen、bounded error 与 retry count。
- Remote Import writeback failure 必须在**同一个**第二短 Redb transaction 中 upsert exact
  typed fault，并 CAS matching stored receipt `Pending -> Degraded`。任一 table open、decode、
  exact compare、insert 或 commit 失败都使整笔回滚，receipt 保持 `Pending`。
- Remote Import writeback success 只在第二短 transaction CAS `Pending -> Written`。`Written`
  与 `Degraded` 均为终态；不得互相改写。相同 exact settlement 可幂等返回既有 receipt。
- 启动 health scan 必须逐个 local repo 读取该表；missing required table、unsupported value
  version、RepoId mismatch 或坏 payload 必须 fail-closed，不能降级为“无故障”。
- repair/rebuild 只有在 exact revalidation 与 materialization 成功后才能删除对应 pending fault。
  历史 Remote Import receipt 已为 `Degraded` 时保持不变；清除 active fault 表示当前 projection
  已修复，不改写历史提交结果。
- watcher start/worker/overflow/final-reconcile failure、Remote Import pre-commit failure、session
  `Stale/Failed` 与 `cleanup_pending` 都不得写入本表。

`ledger/.host/projection-faults.toml`、host-wide journal mutex、dual write 与旧读取 adapter 均
不属于批准实现。未发布 v4 database 缺少该 required table 时按 incomplete schema fail-closed。

## 10. Forbidden Patterns（projection）

> 跨层禁止项见 [index.md](./index.md)。

- 让 Projection Workspace 作为真值源。

## 11. Runtime Boundary（projection 部分）

### 11.2 Projection / Workspace Layer

- 负责由 ledger fold 派生 projection、workspace writeback、projection cleanup 与 drift 解释。
- projection 失败不得伪装成 authority 成功。
- 内部 `projection_locator_runtime` 负责 host-local locator 查询、命令与 admission；父 runtime
  通过 typed boundary 消费其结果，不得直接拥有 locator 文件。

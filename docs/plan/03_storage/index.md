# 03_storage/index.md - Ledger、Projection 与 Workspace 存储工程蓝图（总骨架）

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.2`
- `Last Review`: `2026-07-22`
- `Counterpart Feature`: `docs/features/04_storage.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/07_storage_repo.md`
- `Primary Code Areas`: `crates/core/src/ledger/`, `crates/core/src/ledger/manager/`, `crates/core/src/sync/watcher/`, `crates/core/src/sync/materialize.rs`

> **本章已按 §12 Refactor Target 拆分为四个 runtime 子文件**：
> [authority](./authority.md) · [projection](./projection.md) · [watcher](./watcher.md) · [repair](./repair.md)。
> 本文件承载章节骨架、总览实体、物理布局与跨层边界；各 runtime 专属合同见对应子文件。
> `projection_locator_runtime` 是 `projection_persistence_runtime` 内独立命名的 host-local
> 子 runtime，不增加第五个 storage authority / infra 层；其所有权与禁止跨界见
> [projection.md#projection-locator-contract](./projection.md#projection-locator-contract)。

## 1. Scope

本章定义：

- Repo authority 如何持久化到 ledger。
- Workspace、Tree、Snapshot、Source Control side tables 如何作为 projection 或 workflow side tables 存在。
- Watcher、projection writeback、repair、backup 如何在存储层协作。

本章不描述按钮、列表、提示语或用户操作示例；这些属于 `docs/features/04_storage.md`。

## 2. Authoritative Entities

### 2.1 Core Stores

- `Store A: Projection Workspaces`
  - `ProjectionLocator(RepoId) -> (projection_base, workspace_segment)`
  - `ProjectionWorkspaceRoot(RepoId) = projection_base/workspace_segment`
  - 是 repo-scoped workspace projection 的物理容器，不是 authority。
  - 系统不再定义总 `vault` 根目录；每个本地可写 repo 必须显式绑定 projection base，再计算 repo workspace root。
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

> `Content Facts` / `Structure Facts` 的事实分区与非权威 runtime state、Clean File Policy 见 [authority.md#facts-partition](./authority.md#facts-partition)（§2.3–§2.5）。

## 3. Physical Layout

### 3.1 Ledger Layout

Host runtime composition MUST resolve the configured `ledger_dir` to one canonical absolute
directory before opening any RepoId-scoped authority, catalog, locator, alias, or lifecycle owner.
Relative configuration remains valid and is resolved exactly once against the process working
directory; downstream owners MUST NOT retain or independently reinterpret the relative spelling.

- `ledger/local/<repo_id>.redb`
- `ledger/remotes/<peer_id>/<repo_id>.redb`
- `ledger/.host/identity.key`
- `ledger/.host/projection-locators.toml`
- `ledger/.host/repo-aliases.json`
- `ledger/.host/repo-catalog.lock`
- `ledger/.host/repo-authority-locks/<repo_id>.lock`
- `ledger/.host/repo-catalog/<repo_id>.json`
- `ledger/.host/repo-lifecycle-jobs/<request_id>.json`
- `ledger/.host/repo-lifecycle-jobs/removals/<preparation_id>.json`

每个 local-authority Redb v4 database 内还包含 host-local、非同步的
`projection_faults` recovery side table。它与 Remote Import workflow rows 共处同一
repo database 只是为了获得短事务原子性，不把 Projection Fault 升格为 Ledger Fact 或第四个
authority store；唯一 mutation contract 见
[projection.md#durable-projection-fault-contract](./projection.md#durable-projection-fault-contract)。

Physical database filenames use UUID identity. The canonical execution stem is
the matching lowercase `<repo_id>` string. The current schema-shaped
`RepoInfo.name` also contains that same canonical machine string; it must never
store a human creation label or current alias. The current human-facing alias
belongs to the host-local `HostRepoAliasBinding` store and never determines a
database filename. Projection Locator persists an immutable `workspace_segment`; once
created it is independent from the current alias and cannot be rewritten by an
alias import or rename. A marker, locator or metadata `RepoId` mismatch fails
closed and must not rename the DB, rewrite either side heuristically, or turn a
display string into authority.

`repo-aliases.json` 是 host-local runtime store，不是 user export 文件。写入必须使用
same-directory temp file + flush + atomic replace，读取必须执行 bounded JSON/version/duplicate
校验。其 normalized durable rows 还包含 `alias_revision`；用户导出的 deterministic JSON v1
只投影 `repo_id + alias`，不能反向覆盖 revision 或创建 repo。唯一 mutation/API contract 归
`04_repository#host-repo-alias-contract`。

`repo-catalog/<repo_id>.json` 是 `RepoCatalogRuntime` 独占的 host-local normal-membership
authority record，不是 Ledger fact，也不进入 sync。create/remove 只在短 `Catalog -> Repo`
authority cut 内原子发布单个 bounded record；DB、locator 或 workspace artifact 的存在不能替代
该 record，目录扫描也不得把未登记 artifact 自动 admission 为 normal repo。唯一 lifecycle 合同见
`04_repository#repo-lifecycle-coordinator`。

`repo-catalog.lock` 是同一 host ledger 下所有 `RepoManager`/进程共享的 project-owned advisory
authority lock。它必须以 no-follow regular handle 打开并在加锁后复核 pathname identity；同进程
runtime mutex 与该 file lock 的固定顺序是 process mutex -> file lock。未取得 file lock 的调用不得
读取 conditional cut truth、清理 crash temp、发布 catalog record 或完成 bootstrap seed。

`repo-authority-locks/<repo_id>.lock` 是 `authority_storage_runtime` 按 exact RepoId 使用的稳定
跨进程协调身份。该空文件不包含 repo 数据、永不进入 removal manifest，也不得在 retire 后 unlink；
只有其 OS lock handle 具有排他语义。slot 必须在打开 canonical DB 之前取得该 handle，并持续持有到
DB 关闭/删除、owner cleanup、catalog tombstone retirement 与`TerminalCandidate(publication disabled)` fsync
全部完成后才释放；terminal receipt/publication enablement与session/network delivery位于Retired/lock release之后，
失败只形成control-plane delivery debt。
same-process later same-RepoId admission只能从带expected lock identity的live `Retired`进入
`Reopening -> ReopeningPrepared`：existing-only/no-follow取得并exact复核persistent lock，owner准备fresh DB，
composition绑定DB/lock/locator/marker identity，fresh Normal commit后在固定锁序的composed activation guard中
exact-CAS新generation。unknown cut按durable Normal truth分类；fully removed host restart若没有live Retired proof
不得仅凭pathname或旧receipt自动readmit；
server 已持有该锁时，CLI只能通过 authenticated loopback proxy 使用同一 runtime，不得另开数据库或
把 OS 删除失败当作排他协议。

每个 catalog record 的 deterministic JSON v2 至少包含
`format="deve.host-repo-membership"`、`version=2`、exact `repo_id`、
`state="normal|removed"`、单调 `membership_revision`、prepared identity digest 与最近一次
`lifecycle_request_id`；`removed` 还必须包含 exact `removal_manifest_digest`，`normal` 禁止携带该字段。
文件名、payload RepoId 与 DB identity 任一不一致必须 fail-closed；未发布的 v1 record 不保留 adapter。
`removed` 只是在 ownership-aware `RemoveLocalRepo` 已线性化但 cleanup 尚未全部收敛期间的
transient tombstone；Remote Import owner 已完成本次 removal plan，且成功删除 exact local DB、
workspace `.notegit`、locator 与 alias 后，必须由 catalog owner 删除该 record。它不是可恢复的长期软删除状态。
`repo-lifecycle-jobs/<request_id>.json` 是普通 create/job 的 host-local admission/completion receipt，
记录 operation、normalized intent digest、target RepoId、phase 与 terminal/repair outcome。
Remove 使用 `repo-lifecycle-jobs/removals/<preparation_id>.json` 的 deterministic JSON v4；其
`format="deve.host-local-repo-removal"`、`version=4`、filename stem 与 payload `preparation_id`
必须exact一致，另存非空且互异的`prepare_request_id`，Execute后再存非空且与前两者互异的
`execute_request_id`。请求命名空间在启动时从全部普通receipt与removal record重建，任一重复即
fail closed。v4至少固定exact ownership manifest/digest、confirmation-token hash/expiry/issuer binding、
tagged admission state (`Prepared | Superseded | ExecuteAdmitted`)、独立单调cut state
(`NotAttempted | Attempted | Observed{tombstone}`)、三个owner opaque checkpoint与独立单调terminal
state (`None | Candidate{completion} | Complete`)，以及可选的一次性repair token hash、五分钟expiry、
execution digest与exact owner observation digest；任何checkpoint推进必须原子废止repair授权。cut和terminal不是互斥phase，`Candidate/Complete`
必须隐含`Observed`与`CleanupComplete`。Execute 必须在同一 durable record 内原子转换为
`ExecuteAdmitted { execute_request_id, job_id, consumed_token_hash, ... }` 并 fsync 后才启动 worker；原始
confirmation token 不得落盘；未发布的 v1/v2/v3 removal record 不保留 adapter。它不授予 repo membership，
也不得进入 Ledger/sync。active/cleanup-debt receipt 永不裁剪；normal repo 的 create
receipt 至少由 catalog record 可追溯，terminal receipt 的 bounded retention 归
`04_repository#repo-lifecycle-coordinator`。

当前 `backups/projection-workspace` 只是在 `recover` 命令中由 operator 指定/提供的外部恢复输入，
没有 project-owned RepoId ownership manifest，也不是本章管理的 local-backup runtime。普通
`RemoveLocalRepo` 必须保留所有位于 reserved removal roots 之外的该类输入；若 active recovery input
与 exact `.notegit`、canonical Redb 或 owner-issued Remote Import capture target 重叠，Prepare 必须
返回 blocker。首发不得为了删除功能临时新建 managed Ledger backup authority。

### 3.1.1 Remote Import Runtime Layout {#remote-import-runtime-layout}

Remote Import capture is host-only, immutable runtime state:

```text
ledger/.host/remote-imports/<repo_id>/<session_id>/
  source-manifest.json
  candidates/<revision>.json
  blobs/<sha256>
ledger/.host/remote-imports/<repo_id>/.removal-plan.json
ledger/.host/remote-imports/.deve-removing-<quarantine_id>-<repo_id>/
```

- `source-manifest.json` is deterministic JSON schema v1; `blobs/<sha256>` are
  content-addressed immutable bytes and candidates are deterministic projections
  of that sealed snapshot.
- This directory is not a fourth authority store. It must not be synced,
  materialized as Projection content, watched as workspace input, exposed to the
  browser, or interpreted as Source Control/External Changes state.
- Redb owns session identity/state and exact digests; the filesystem owns only
  the sealed payload bytes. Neither side may infer the missing half from names
  or timestamps.
- `.removal-plan.json` 是 `remote_import_runtime` 在 remove pre-cut 阶段维护的per-RepoId single
  owner-only durable plan slot。通用 lifecycle receipt 只保存其 opaque path identity、logical epoch与
  content digest，不内嵌逐文件清单。相同job重试必须exact复用；新job只能在provider quiesced时以
  atomic replace+parent sync发布更高logical epoch，故每repo最多一个sidecar。pre-cut补偿只使当前epoch
  逻辑失效，不执行pathname删除；失效slot由下一次seal原子替换或由最终whole-root cleanup收敛。Removed cut后，owner必须把exact `<repo_id>` artifact root
  no-replace移动到同父、manifest-bound `.deve-removing-*` quarantine，复核moved identity并sync parent，
  再删除整个quarantine root；不再逐项unlink inventory path。sidecar、quarantine path与identity不得进入
  sync、browser、session state或正常capture listing。

### 3.2 Repo Runtime Layout {#repo-runtime-layout}

- `<projection_base>/<workspace_segment>/.notegit/`
  - repo keys
  - pending/staged side tables
  - commit/runtime metadata
  - migration archives

约束：

- `.notegit/` **MUST** 被 watcher 忽略。
- `.notegit/` 可以随 repo 备份，但 **MUST NOT** 被跨 repo 复用。
- `.notegit/` 是 Deve-owned repo runtime 目录，当前继续保留该命名。
- Remove Prepare 为 `.notegit` tree、identity marker与canonical Redb冻结同父、manifest-bound
  quarantine pathname；这些保留名只属于exact removal job。quarantine不是backup/recycle bin，不能被
  watcher、normal startup、repo discovery或repair scan当成可admit repo对象。
- ownership-aware `RemoveLocalRepo` 只能删除 workspace root 下 exact、identity-matched 且自身非
  symlink/junction/reparse point 的 `.notegit/` 树；workspace root 与其它 child（包括 Markdown、附件、`.git/`、
  `.gitignore`、`.deveignore`）全部保留。no-follow walker 可以删除 child link/reparse entry 本身，
  但不得解析或进入 target；顶层 `.notegit` identity replacement 永远阻断自动 removal/repair。

> Projection Locator Layout（projection-locator-contract）见 [projection.md#projection-locator-contract](./projection.md#projection-locator-contract)（§3.2.1）。

### 3.2.2 Git Mirror Storage Boundary {#git-ecosystem-coexistence}

Git mirror 的生命周期、命令面与失败语义以 `05_diff_logic.md#git-mirror-lifecycle` 为唯一权威。本章只定义存储边界：

- `.notegit/` 是 Deve-owned runtime 目录，保存 ledger-aware workflow state 与必要 side table。
- `.git/` 是 Git ecosystem mirror 目录，只用于复用 Git 工具链、远程托管、审计、备份与发布生态。
- repo-local `.gitignore` **MUST** 忽略 `.notegit/`，避免 Git mirror 泄漏 Deve runtime state。
- watcher、scan、sync 与 projection rebuild **MUST** 按路径段语义忽略 `.notegit/` 与 `.git/` 内部路径。
- Deve core **MUST NOT** 使用 `.git/` 作为 repo authority、runtime side table 或 hidden metadata 目录。
- `.git/` 不得参与 ledger fold、repo scope 解析、stage/commit authority 或 repair。

### 3.3 Collision Rules

- 同一 branch 下 `.redb` 文件名必须是 canonical `RepoId`；同名 display alias
  不产生物理冲突。
- 非 UUID filename、同一 `RepoId` 的重复文件或 identity mismatch 必须
  fail-closed 并进入显式 catalog/repair；不得自动重命名后猜测归属。

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

浏览器能力探测必须保持三层边界：JS bridge 只调用平台 API 并返回能力事实；
`browser_identity_client` 把能力事实归一化为强类型 write blocker；UI 只把 blocker
投影为本地化只读原因与恢复指引。`localStorage` 不属于 writer identity 前置条件。
WebCrypto、IndexedDB 或 WebCrypto Ed25519 任一必需能力缺失时必须 fail-closed；不得由
JS/WASM polyfill 生成可导出私钥、伪造平台能力或绕过 browser peer 签名。
Android native WebView 的正式可写 evidence baseline 是 API 29+ 与当前 provider major 137+；
版本事实只属于 support/receipt 诊断，最终 writer identity 仍必须由上述真实 non-extractable
Ed25519 probe 证明。技术上通过 probe 但低于支持基线的设备不得生成正式 target-host receipt，
而缺失 probe 的任何设备都不得因版本号、native session 或 host capability 获得写权限。

> Browser storage recovery semantics（站点数据被清理、IndexedDB/WebCrypto 缺失降级）见 [repair.md](./repair.md)（§3.4.2 Recovery Semantics）。

### 3.5 Internal Path Normalization {#internal-path-normalization}

- 所有持久化到 ledger、projection table、side table、sync payload 的路径字符串 **MUST** 统一为 forward slash。
- 规范化边界：
  - 进入系统：watcher、file dialog、CLI path 参数进入 authority/runtime 前，必须调用 `to_forward_slash`。
  - 离开系统：仅在直接调用 OS 文件系统 API 的瞬间，才允许转换回 native separator。
- 禁止：
  - 在不同表中混用 `\` 与 `/`
  - 依赖 display path 作为 authority key
  - 通过字符串替换拼接路径身份

## 5. State Machines

### 5.1 Repo Mount Lifecycle

```text
RepoDiscovered
  -> RepoOpened
  -> RuntimeTablesReady
  -> ProjectionLocated
  -> ProjectionReady
  -> WatcherTransitioning(generation)
  -> WatcherReady
  -> Mounted(generation)
```

约束：

- `ProjectionLocated` 必须验证 repo-scoped Projection Locator。
- `WatcherReady` 是打开 repo 的最后一步；只有 owned watcher handle 已完成 capture-first clean scan cut，才允许发布 `Mounted(generation)`。
- durable `RepoHealth` 与 process-local `RepoMountState` 正交。依赖 Projection Workspace 当前性的在线本地写路径只有在下式成立时才可准入：

  ```text
  RepoHealth::Healthy && RepoMountState::Mounted
  ```

- watcher 初始化或运行期失败必须对该 repo fail-closed 为 `RepoMountState::Failed`，不得写入 projection fault journal，也不得伪装为 `DegradedProjection`。
- server bootstrap 对 repo-local watcher start failure 按 repo 隔离；健康且成功 mounted 的 repo 继续运行。零个 local repo 时，host 以 `NoScope` 正常启动且 watcher `expected=0` 为 healthy；存在 local repo 但零个 `Mounted` 时，host 仍保留只读、诊断与 Create 能力。只有 typed supervisor/runtime host-fatal 才回滚已启动 watcher 并终止 server。
- server 已运行后即使全部 watcher 后续失败，仍保留纯读、ledger inspect/export 与离线 repair/diagnostic 能力；所有依赖 workspace 当前性的在线本地 mutation 保持关闭。
- `Transitioning / Mounted / Failed` 的 generation、失败原子切点与 owned lifecycle 唯一由 [watcher contract](./watcher.md#watcher-contract) 定义；repo create/remove 的 durable 与 mount 协调唯一由 `04_repository#repo-lifecycle-coordinator` 定义。host-local alias不参与 mount lifecycle。

> Write Lifecycle（§5.2）见 [authority.md](./authority.md)；External Edit Lifecycle（§5.3）见 [watcher.md](./watcher.md)。

## 10. Forbidden Patterns（跨层）

以下为跨 runtime 的禁止项；各 runtime 专属禁止项见对应子文件的 Forbidden 段：

- 原地修改 authority 状态。
- 通过全局 `vault_path` 或 `ledger_dir` 隐式推断 repo projection base / workspace root。

各 runtime 专属禁止项：

- authority 专属（metadata/path table 直接 rename/move/delete、未经 Stage/Commit 入 ledger）见 [authority.md](./authority.md)。
- projection 专属（Projection Workspace 作为真值源）见 [projection.md](./projection.md)。
- repair 专属（side table 或 snapshot 成为删除真源）见 [repair.md](./repair.md)。

## 11. Runtime Boundary

各 runtime layer 的完整边界分布在子文件；本节给出总览与 Repo Runtime Integration。

- §11.1 Authority Layer → [authority.md](./authority.md)
- §11.2 Projection / Workspace Layer → [projection.md](./projection.md)
- §11.3 Watcher Layer → [watcher.md](./watcher.md)

### 11.4 Repo Runtime Integration

- 负责 repo open/close、runtime directory bootstrap、catalog repair 与各层生命周期编排。
- 该层只能编排 authority/projection/watcher/repair，不得把 side table 升格为 authority。

## 12. Refactor Target

长期应显式形成四个 infra 子系统（本章的文件夹化即按此四层落地）：

- `authority_storage_runtime` → [authority.md](./authority.md)
- `projection_persistence_runtime` → [projection.md](./projection.md)
- `watcher_runtime` → [watcher.md](./watcher.md)
- `repair_runtime` → [repair.md](./repair.md)

实现必须按这四层收敛；任何 manager/helper 只能作为其中一层的内部细节，不得跨层持有隐式 authority。
`projection_locator_runtime` 作为 `projection_persistence_runtime` 的独立子 runtime，唯一拥有
host-local locator 文件；该父子关系不得被解释为新的顶层 storage authority 层。

浏览器侧另设 `browser_identity_client` 作为本章 storage layering 的 client adapter；
它只归一化平台 capability、管理 non-extractable browser identity readiness，并向 UI
投影强类型 blocker，不是第五个 storage authority runtime。

## 本章相关命令

- 无

## 本章相关配置

- `snapshot_depth`
- backup / retention 相关配置
- `projection.locators`
- `ledger.path`

# 05_diff_logic.md - Diff 与 Source Control 工程蓝图

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-06-15`
- `Counterpart Feature`: `docs/features/07_diff_logic.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/04_diff.md`
- `Primary Code Areas`: `crates/core/src/source_control/`, `crates/core/src/ledger/source_control.rs`, `apps/cli/src/server/handlers/source_control/`, `apps/web/src/hooks/use_core/callbacks_sc_*.rs`

## 1. Scope

本章定义类 Git 工作流在 Deve-Note 中的工程实现合同。

术语边界：本章的 `stage / commit / diff / merge` 是 Source Control 交互语义，
不是 Git object store、Git index 或 `.git/` 目录的 authority 承诺。核心
authority 仍是 ledger facts 与 commit anchors。

本章只处理三类问题：

1. 工作区差异如何从文件系统事件进入 `pending_fs_ops`。
2. Stage / Commit / Diff / Merge 如何回到 ledger authority。
3. Source Control runtime 如何与 document runtime、repo scope runtime 分层协作。

跨 repo 自动合并、显示层交互细节与按钮文案不属于本章。

强约束：

- 本章定义的 diff / merge 仅适用于同一逻辑 repo。
- `RepoId` 不一致时，任何自动 merge 都必须视为 undefined behavior 并 fail-closed。
- Core Source Control **MUST NOT** 通过 Git CLI、`.git/objects`、Git index 或 Git refs
  判定 Deve commit 是否成立。
- Git mirror 是一等生态桥接层，但只能作为 projection export/import/publish 适配层；
  导入必须生成 Deve ledger facts，导出不得反向改写 ledger authority。
- Git index 只允许在 mirror update 的最后提交阶段使用，**MUST NOT** 替代 Deve staging。

## 2. Authoritative Entities

### 2.1 Diff Domains

系统必须显式区分两个差异域：

- `Working Directory Domain`
  - 来源：repo Projection Workspace 与当前规范 projection 的偏差
  - 状态：`pending_fs_ops`
- `Ledger Commit Domain`
  - 来源：已确认 ledger facts 与 commit anchors
  - 状态：`staged`, `commits`, `commit_diff`

### 2.2 Core State

- `PendingFsEntry`
- `StagedEntry`
- `CommitInfo`
- `CommitAnchor(ledger_seq)`
- `DiffSession`
- `MergeBase`
- `ConflictSet`

### 2.3 Authority Rule {#authority-diff-core}

- 文本差异最终必须收敛为 `Content Facts`。
- rename / move / create / delete 最终必须收敛为 `Structure Facts`。
- `pending_fs_ops`、`staging`、`diff cache` 都不是 authority。
- Git mirror commit 不是 authority；它只是已确认 Deve commit 的外部生态映射。

### 2.3.1 Git Mirror Lifecycle {#git-mirror-lifecycle}

Git mirror 生命周期必须挂在 Source Control authority 之后：

```text
DeveStaged
  -> DeveLedgerCommit
  -> ProjectionPersisted
  -> GitMirrorQueued
  -> GitMirrorCommitted | GitMirrorOutOfSync
```

- `DeveLedgerCommit` 一旦成功，不得因为 Git mirror 失败而回滚。
- `source_control.git_bridge = "mirror"` 是兼容默认值：`DeveLedgerCommit` 成功后，
  如果 `.git` mirror ready，则 **MAY** 排队 `GitMirrorQueued` record；后续显式
  `deve git mirror/export/push` 仍必须经过本节 preflight。
- `source_control.git_bridge = "off"` 表示 Deve-only / NoteGit-only 运行：stage、
  commit、diff、history 与 merge 仍完全走 ledger authority；commit 成功后
  **MUST NOT** 排队 Git mirror record，也不得执行 Git bridge 写操作。
- `source_control.git_bridge` 是 source-control writer 的显式 runtime policy；CLI `sc commit`、
  HTTP commit、plugin-host HTTP mutation 与 Rhai `sc_commit` 等所有 commit writer surface
  必须接收同一个 effective policy，不能通过默认 `mirror` 入口绕过 `off`。
- Core Source Control writer API **MUST NOT** 暴露无 policy 的 commit 方法；新增或重构的
  commit writer 必须在编译期显式选择 `GitBridgeMode`，delegated remote proxy 只能在独立
  delegated path 内转发到主进程 effective policy。
- `off` 模式下，`deve git status` 与 `deve git import` dry-run 可作为只读诊断；
  `deve git mirror/export/import --apply/push` 必须返回结构化 disabled blocker，并提示
  需要将 `source_control.git_bridge` 切回 `mirror`。
- `GitMirrorOutOfSync` 必须能被 `status` / `repair` / retry 路径观测。
- 外部 Git 操作造成的工作区变化进入 `pending_fs_ops` 或显式 `GitImportRequested`，
  不得直接修改 `CommitAnchor`、`StagedEntry` 或 ledger facts。

Git mirror 命令面必须遵守以下边界：

- `git status` 只能观测 mirror readiness、queue state 与 out-of-sync 诊断，**MUST NOT** 写 `.git/`、`.notegit/` 或 ledger。
- `git mirror` / `git export` **MAY** 把已完成的 Deve commit projection 导出为 Git commit；导出失败只能产生 `GitMirrorOutOfSync`，**MUST NOT** 回滚 Deve commit。
- `git import` dry-run 只能生成 change/blocker plan；默认 **MUST NOT** 写 ledger、`pending_fs_ops`、`StagedEntry`、`CommitAnchor` 或 `.notegit/`。
- `git import --apply` 只能把安全 changes 写入 pending/import，并保留冲突标记；后续 **MUST** 通过 Deve stage/commit 生成 ledger facts。
- Git bridge 写命令（`git mirror/export/import --apply/push`）在写 pending/import、`.git` mirror
  或发布 mirror HEAD 之前 **MUST** 复用本地 Projection Workspace identity gate；
  Projection Locator 或 `.notegit` identity marker 破损时必须 fail-closed。
- `git push` 只能发布已映射的 `.git` mirror HEAD；它 **MUST** fail-closed 于 mirror 未 ready、Source Control 不干净、Git worktree 不干净、存在 queued/out-of-sync record、`.notegit` tracked 泄漏或 Git HEAD 未映射到最新 Deve commit。
- repair action schema 只能用于诊断、人工修复指引和显式 retry；**MUST NOT** 被 Web、后台任务或 Command Palette 解释为自动 Git 写入授权。
- 自动后台执行、可点击 repair UI 与 Web 后端直接执行 Git 写入 **MUST** 作为独立设计批次处理，不能从只读 status/review surface 隐式升级。
- Proxy / plugin-host node role 摘要必须展示主进程 effective `source_control.git_bridge`
  或明确的 delegated/unknown 状态，不得硬编码为 `mirror` 并误导 Web surface。

### 2.4 Diff Identity Model

- 文本 diff 身份：
  - `DocId`
  - UTF-16 position space
- 结构 diff 身份：
  - `NodeId`
  - `DocId`
- UI 中的 `path` 只是 selector/display，不得作为长期 merge identity。

## 3. State Machines

### 3.1 External Edit Lifecycle

```text
ProjectionWorkspaceChangeDetected
  -> PendingFsEntry
  -> Staged
  -> LedgerCommitted
  -> Cleared
```

旁路：

```text
PendingFsEntry -> Discarded
PendingFsEntry -> Conflict
Staged -> Unstaged
```

约束：

- Watcher 检测到的变更 **MUST NOT** 直接写入 ledger。
- `Stage` 是 repo-scoped side-table 迁移，不是 UI 样式变化。
- `Discard` 的语义是恢复 workspace 到当前规范 projection。
- 普通 `Stage` **MUST** fail-closed 于 `has_conflict=true` 的 pending entry；
  只有显式 `ResolveConflict(KeepFs)` flow 可以通过 resolved-stage 路径清除 conflict 标记并移入 staged。

补充：

- watcher **MUST** 忽略 `.notegit/`
- watcher、启动扫描、目录重扫 **MUST** 统一应用 `.deveignore`；被忽略路径不得生成 pending / staged / ledger diff。
- `pending_fs_ops` **MUST** 表示 Working Directory Domain
- `staging` **MUST** 表示用户确认后的过渡域，而非已提交历史

### 3.2 Commit Diff Lifecycle

```text
CommitSelected
  -> CommitDiffRequested
  -> CommitDiffReady
  -> DiffSessionBound
  -> Closed
```

约束：

- diff session 是只读派生状态，不得反向改写 source control authority。
- `path` 的 canonical identity 与 `display_path` 必须分离。

补充：

- rename/move diff 在 session 中至少要保留：
  - canonical path identity
  - display path
  - counterpart path / successor path
- 不得把 `"old -> new"` 这样的 display label 回灌为 canonical identity

### 3.3 Merge Lifecycle {#merge-contract}

```text
MergeRequested
  -> LCAResolved
  -> DiffCalculated
  -> AutoMerged | Conflict
  -> ConfirmedResult
  -> LedgerCommitted
```

约束：

- merge 只允许发生在同一逻辑 repo 内。
- cross-repo merge **MUST** fail-closed。
- merge runtime 只能由本机 local writer 发起；remote mirror / shadow branch 是只读输入，不是并发写者。
- `MergePeer` 产出的任何写入都必须重新进入本机 repo-scoped writer gate，并作为新的 local ledger facts 提交。

冲突检测原则：

- `Diff_local = Base -> Local`
- `Diff_remote = Base -> Remote`
- 修改区域不重叠时 **MAY** auto-merge
- 修改区域重叠时 **MUST** 进入 explicit conflict state

## 4. Commands / Inputs / Outputs

### 4.1 Inputs

- `StageFile`
- `StageFiles`
- `UnstageFile`
- `DiscardFile`
- `DiscardPending`
- `CommitStaged`
- `RequestChanges`
- `RequestCommitHistory`
- `RequestCommitDiff`
- `RequestDocDiff`
- `MergePeer`
- `ResolveMergeConflict`

### 4.2 Output Contracts

- `ChangesList`
- `StageAck`
- `UnstageAck`
- `DiscardAck`
- `CommitAck`
- `CommitHistory`
- `CommitDiffResult`
- `DocDiff`
- `MergeConflict`
- `MergeComplete`
- `ProtocolError`

### 4.3 Input Safety

- 所有 source control 请求 **MUST** 带 `scope_nonce`。
- Browser HTTP source-control mutation 请求仅携带非零 `scope_nonce` 不构成写授权；`scope_nonce`
  必须匹配当前 authenticated browser session 通过 `SyncHello + RegisterWriter` 获得的
  server-side `SourceControlWriteGrant`。
- 所有 remote branch source control 请求 **MUST** 经过 readonly gate。
- 本地 source-control 写入口（包括 CLI `deve sc stage/commit`、plugin-host HTTP mutation 与 Rhai `sc_commit`）
  在写 pending/staging/commit 之前 **MUST** 验证 Projection Locator 与 `.notegit`
  identity marker 仍绑定同一 local repo；破损或漂移时必须 fail-closed。
- Web Source Control 的只读 diff / history / graph 入口 **MUST** 使用 read gate；
  remote / spectator scope 只能隐藏或禁用写操作，不得用 write gate 阻断只读 diff 查看。
- path-only target 仅允许作为 selector 输入，落到算子前必须解析为文档/节点 identity。
- 唯一兼容例外是 legacy `Deleted + doc_id=None` 的 exact delete selector：stage/discard wrapper 可保持 path-only，但 commit delete planning **MUST** 再通过当前 node projection 解析目标 identity；非 delete 的 docless tracked entry 不得使用该例外。

### 4.4 Output Payload Minimums

- `ChangesList`
  - `repo_id`
  - `branch`
  - `scope_nonce`
  - `pending/staged entries`
- `CommitDiffResult`
  - `repo_id`
  - `commit_id`
  - canonical targets
  - display labels
- `DocDiff`
  - `doc_id`
  - base/target identifiers
  - diff hunks in UTF-16 compatible space
- `MergeConflict`
  - `repo_id`
  - `branch`
  - `scope_nonce`
  - `doc_id`
  - `path`
  - `current_content`
  - `incoming_content`
  - `result_content`
  - 可用 resolution actions
  - 结构化 conflict hunks

### 4.5 Diff View Output Contract

- `Side-by-Side`
  - 默认 commit/doc diff 视图结构。
  - 左右两侧必须绑定稳定的 base/target identity，不得因为滚动或 rename label 改变 identity。
- `Gutter Indicators`
  - 编辑器左侧槽必须能表达 add / modify / delete / conflict。
  - gutter 状态来源于 diff runtime typed output，而不是 DOM 比较或 CSS 猜测。
- `Inline Diff`
  - 允许在编辑态显示相对 confirmed state 的即时差异。
  - inline diff 仍然是 projection，不得替代 staged/commit history。

## 5. Algorithms and Contracts

### 5.1 Text Diff Contract

- 使用 Myers / patience / fallback 组合是实现选择，不是权威来源。
- 所有 text diff 的位置语义 **MUST** 与编辑器链保持一致；当前以 UTF-16 code unit 为统一索引标准。
- text diff 只能表达 `Content Facts`，不得承担结构变更语义。

补充：

- `ContentOp::Insert` / `ContentOp::Delete` 的索引空间 **MUST** 与 JS/CodeMirror 完全一致。
- 任何 byte-based diff 结果在进入协议前都必须转换到 UTF-16 index space。

### 5.2 Structure Diff Contract

- rename / move / create / delete 的 commit diff 必须基于 `NodeId / DocId` 关联，而不是仅凭路径字符串。
- reused path、rename successor、counterpart path 都必须由 source control target lookup 解析，不得在 UI 层猜测。

补充：

- delete 事实必须来自显式删除结构事实，而不是“文件不存在”这一观察结果本身。
- rename / move 若无法解析稳定 identity，必须作为结构冲突或 reject，而不是静默退化成 delete+create。

### 5.3 Commit Contract

`CommitStaged` **MUST** 以如下路径完成：

1. 读取当前 repo 的 confirmed projection。
2. 对 staged entries 计算内容与结构差异。
3. 生成 `Content Facts` / `Structure Facts`。
4. 追加到 ledger。
5. 生成 commit record，锚定结果 `ledger_seq`。
6. 清理已消费的 staged / pending 条目。
7. 重建或增量更新 projection。

### 5.4 Merge Contract

- `Base` = LCA snapshot / anchor。
- `Left` = local confirmed state。
- `Right` = remote mirror confirmed state。
- auto-merge 仅允许在差异域不重叠时发生。
- conflict result 必须显式进入 resolution flow，不得静默写盘。

补充：

- `MergePeer` 的最终写入只能进入 Local Branch。
- Remote mirror 自身不得因 merge 流程被直接改写。
- merge 不是远端并发编辑协议；它是本机对 remote mirror snapshot 的显式读入、计算与本地提交。
- merge result 若需要用户选择，结果必须在确认后才允许落 ledger。

## 6. Failure Modes

错误码清单以 `13_i18n.md#i18n-error-code-catalog` 为唯一权威；本节只列失败域。

### 6.1 Pending / Staging Failures

- watcher overflow
- ambiguous path target
- missing staged workspace file
- staged entry doc identity missing

### 6.2 Commit Failures

- workspace file not readable
- structure diff cannot resolve stable identity
- ledger append rejected
- projection writeback failed

### 6.3 Merge Failures

- no common ancestor
- cross-repo merge request
- structure conflict unresolved
- remote scope not writable but merge requested

### 6.4 Conflict Resolution Output Contract

- 冲突结果至少要能表达：
  - `Current(Local)`
  - `Incoming(Remote)`
  - `Result`
  - `AcceptCurrent`
  - `AcceptIncoming`
  - `AcceptBoth`
- 这些是 resolution runtime 的 typed outputs，不是 view 文案常量。
- 默认冲突展示 **SHOULD** 支持 side-by-side 视图。
- inline resolution 仅作为同一 conflict model 的另一种展示，不得形成第二套 conflict authority。
- 运行时协议使用 `ServerMessage::MergeConflict` 作为 typed conflict model。
- 运行时确认使用 `ClientMessage::ResolveMergeConflict`；它不同于 source-control `ResolveConflict`。
- `DocDiff` **MAY** 作为兼容 fallback 继续输出，但 **MUST NOT** 成为 conflict authority。

## 7. Recovery / Repair

### 7.1 Working Directory Recovery

- `Discard` **MUST** 从当前 projection 恢复，不得从 stale UI buffer 恢复。
- watcher overflow **MUST** 先触发 reconcile scan，再继续处理增量事件。

### 7.2 Source Control Table Repair

- local repo source control tables may be repaired as infra maintenance.
- repair 只能修 pending/staging/commit indexes，不得伪造 ledger history。

### 7.3 Commit Diff Recovery

- path target 解析失败时，系统 **MUST** 返回结构化错误，不得静默降级到错误文档。
- 如果 commit diff 结果不可用，UI 只能进入明确的 unavailable state，不得显示伪 diff。

### 7.4 Watcher Overflow Recovery

- overflow 后 **MUST** 先进行 reconcile scan，再恢复增量处理。
- reconcile 期间不得继续基于旧 pending 集合生成 commit diff。

## 8. Forbidden Patterns

- 让 watcher 直接写 ledger。
- 用 path string 作为长期 diff/merge identity。
- 让 UI 直接修改 staged/pending side table。
- 让 commit diff 与 doc diff 走两套不兼容 identity 规则。
- 让 remote readonly branch 暴露可写 source control 行为。
- 把外部编辑三阶段流程误套用到 Web thin client 默认编辑路径。

## 9. Runtime Boundary

### 9.1 Authority / Diff Core

职责：

- pending/staging/commit indexes
- diff algorithms
- commit anchors
- source control errors

### 9.2 Repo Manager Integration

职责：

- repo-bound source control execution
- target lookup
- workdir reconciliation

### 9.3 Server Runtime {#source-control-runtime}

职责：

- ws/http dispatch
- scope guard
- readonly gating
- browser HTTP mutation write grant

`SourceControlWriteGrant` 是 server runtime 内部状态，不是 wire protocol 字段。Browser WebSocket 在同一
authenticated session 内完成 `SyncHello` 与 `RegisterWriter` 后，server 必须生成短生命周期 grant，至少绑定：

- `auth_session_id`（由当前 cookie/JWT session 的不可逆 digest，或 anonymous localhost dev session
  cookie nonce 的不可逆 digest 派生，不暴露 token material；不得只由 dev-wide 固定值派生）
- `repo_id`
- local writable branch / local-only target
- registered `writer_peer_id`
- `scope_nonce`
- expiry

主进程 `/api/sc/stage-pending`、`/api/sc/discard-pending`、`/api/sc/unstage` 与 `/api/sc/commit`
等 HTTP mutation 在执行 ledger/source-control 写入前，必须同时验证：

- HTTP JWT/session 有效；
- repo selector 解析到 grant 绑定的同一 `repo_id`；
- 目标是 local writable branch；
- 请求 `scope_nonce` 与 active grant 完全匹配；
- grant 仍属于当前 browser session 的 active writer grant，且未过期。

WS repo switch、branch switch、disconnect、session invalid、writer unregister、重新绑定 writer，或任何
repo/scope recovery / sync guard / Browser `SyncHello` failure 路径导致当前 scope runtime binding 被清理时，
server 必须撤销或替换对应 grant。
remote proxy delegated API 不得复用主进程 browser HTTP mutation 语义；它必须走显式 delegated path /
`SourceControlWriteAuthority::DelegatedRemoteProxy` 等等价枚举。`REMOTE_PROXY_SCOPE_NONCE = 1`
只能在 delegated API 内解释，普通主进程 HTTP mutation 不得因为 scope nonce 为 1 而接受写入。
delegated API 还必须具备独立于 browser/JWT cookie 的 server-verifiable capability；
普通 authenticated browser request 即使知道 `/api/delegated/sc/*` 路径，也不得仅凭 cookie/session
触发 delegated writer。
remote proxy 的只读 repo/source-control 查询同样不得依赖 browser JWT 或 anonymous localhost dev
session；它们必须走显式 delegated read path，复用只读 handler，但只授予 query authority，不得隐式
升级为 writer grant。
plugin-host remote delegated Source Control API 必须通过显式 delegated proxy 类型边界注册；普通
`SourceControlApi` 或本地 `RepoManager` 实现不得被登记为 delegated mode，以免跳过本地 Projection
Workspace identity gate 后直接写本地 staging/ledger。
若同一 browser request 同时携带有效 JWT cookie 与 anonymous localhost dev session cookie，HTTP 与 WS
Source Control grant 校验必须共同使用 JWT 派生的 `auth_session_id`；dev session cookie 不能覆盖已登录 session。

### 9.4 Web Runtime

职责：

- request building
- scope nonce binding
- diff session lifecycle
- source control notices

### 9.5 View Layer

职责：

- 展示 changes/history/graph/diff
- 发出 typed intent
- 不得直接操作 side tables 或 repo state

## 10. Refactor Target

长期应将 source control 主线收敛成单独 runtime：

- `source_control_runtime`
- `diff_session_runtime`
- `merge_runtime`

未来重构必须分离 workspace diff 与 confirmed ledger diff；core manager、CLI proxy 与 `use_core` 回调不得共享隐式 source-control 状态。

## 本章相关命令

- `P2P: Merge Peer`

## 本章相关配置

- `diff.merge_strategy`: `manual` | `auto`

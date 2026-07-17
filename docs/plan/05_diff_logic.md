# 05_diff_logic.md - Diff 与 Source Control 工程蓝图

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-17`
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

系统必须显式区分四个差异域：

- `External Changes Domain`
  - 来源：repo Projection Workspace 与当前规范 projection 的偏差
  - 状态：`pending_fs_ops` + External Changes staging
  - 语义：投影文件夹只是待确认输入源；外部文件变化在用户确认前不得写入 ledger。
- `Confirmed Ledger Domain`
  - 来源：最新 commit anchor 之后已经确认的 ledger facts
  - 状态：只读派生 `ConfirmedLedgerChange`
- `Ledger Commit Domain`
  - 来源：已确认 ledger facts 与 commit anchors
  - 状态：`commits`, `commit_diff`
- `Git Main Mirror Domain`
  - 来源：已完成 NoteGit/ngit commit 的外部生态终态映射
  - 状态：main branch mirror queue、out-of-sync diagnostics、只读 status / repair review

### 2.2 Core State

- `PendingFsEntry`
- `StagedEntry`
- `CommitInfo`
- `CommitAnchor(ledger_seq)`
- `ConfirmedLedgerChange`
- `DiffSession`
- `MergeBase`
- `ConflictSet`

### 2.3 Authority Rule {#authority-diff-core}

- 文本差异最终必须收敛为 `Content Facts`。
- rename / move / create / delete 最终必须收敛为 `Structure Facts`。
- `pending_fs_ops`、`staging`、`diff cache` 都不是 authority。
- `ConfirmedLedgerChange` 不是 runtime side table；它只能由 commit anchor 与 ledger head 派生。
- Git main mirror commit 不是 authority；它只是已确认 NoteGit/ngit commit 的外部生态终态映射。
- Source Control commit 是版本锚点创建动作；External Changes 的 `Apply to Ledger`
  只是把 staged external changes 写入 ledger facts，**MUST NOT** 同时创建 commit anchor。
- 若同一文档同时存在 external change 与 confirmed ledger dirty，External Changes
  **MUST** fail-closed：禁止普通 stage / apply-to-ledger，只允许打开 diff 或丢弃外部修改。

### 2.3.1 Git Main Mirror Lifecycle {#git-mirror-lifecycle}

Git main mirror 生命周期必须挂在 Source Control authority 之后：

```text
DeveStaged
  -> NoteGitLedgerCommit
  -> ProjectionPersisted
  -> GitMirrorQueued
  -> GitMirrorCommitted | GitMirrorOutOfSync
```

- `NoteGitLedgerCommit` 一旦成功，不得因为 Git main mirror 失败而回滚。
- Source Control 只有 NoteGit/ngit authority；不再存在 `source_control.git_bridge`
  或 mirror/off 二选一配置。
- NoteGit/ngit commit 成功后，如果 `.git` mirror ready，则 **MUST** 排队
  `GitMirrorQueued` record。排队失败只能形成诊断，不得回滚 ledger commit。
- Git main mirror 只关心 Markdown Projection Workspace 终态与 NoteGit/ngit 终态一致；
  不要求 Git 历史轨迹与 NoteGit/ngit 历史逐条同构。
- Git main mirror writer 采用“终态例外提交”：每次执行 mirror 时提交当前尚未被
  Git main 覆盖的 projection 终态；其它由外部 Git main drift 造成的差异只进入
  diagnostic / External Changes / explicit import 路径，不直接写 ledger。
- `.notegit/` 必须被 `.gitignore` 忽略，且不得被 Git tracked；检测到泄漏必须 fail-closed。
- Core Source Control writer API 不得接收 legacy bridge mode 或 Git authority policy；CLI `sc commit`、
  HTTP commit、plugin-host HTTP mutation 与 Rhai `sc_commit` 均走同一 NoteGit/ngit commit path。
- 当前 v1 不开放 NoteGit/ngit rollback / revert / reset 命令；若未来开放按 commit anchor
  回退 N 步，Git main mirror 的对应回退必须只依赖 mirror mapping metadata，而不是比较提交消息或路径猜测。
- `GitMirrorOutOfSync` 必须能被 `status` / `repair` / retry 路径观测。
- 外部 Git 操作造成的工作区变化进入 `pending_fs_ops` 或显式 `GitImportRequested`，
  不得直接修改 `CommitAnchor`、`StagedEntry` 或 ledger facts。

- ngit mirror status 只能观测 mirror readiness、queue state 与 out-of-sync 诊断，
  **MUST NOT** 写 `.git/`、`.notegit/` 或 ledger。
- ngit mirror/export **MAY** 把已完成的 NoteGit/ngit projection 终态导出为 Git main commit；
  导出失败只能产生 `GitMirrorOutOfSync`，**MUST NOT** 回滚 NoteGit/ngit commit。
- ngit import dry-run 只能生成 change/blocker plan；默认 **MUST NOT** 写 ledger、
  `pending_fs_ops`、`StagedEntry`、`CommitAnchor` 或 `.notegit/`。
- 当 Git name-status 已报告 copy record（`C* old new`）时，ngit import dry-run
  **MUST** 将当前路径 `new` 作为 `Added` change 进入 plan；Git copy source
  `old` 只属于 Git 诊断上下文，不得成为 ledger copy authority 或 `previous_path`。
- ngit import --apply 只能把安全 changes 写入 pending/import，并保留冲突标记；后续
  **MUST** 通过 External Changes / Apply to Ledger / Source Control commit 生成 ledger facts 与 commit anchor。
- Git mirror 写命令在写 pending/import、`.git` mirror 或发布 mirror HEAD 之前
  **MUST** 复用本地 Projection Workspace identity gate；
  Projection Locator 或 `.notegit` identity marker 破损时必须 fail-closed。
- Git bridge 进程在 `DEVE_GIT_EXECUTABLE` 存在时 **MUST** 只执行该变量指向的
  canonical absolute ordinary file；相对路径、不存在路径或目录必须 fail-closed，
  不得再回退到 `PATH`。普通 CLI 未设置该内部绑定时 **MAY** 继续按自身 `PATH`
  解析 `git`。Desktop shell 的受控 sidecar 绑定规则见 Desktop native contract。
- Git executable 缺失或无效只允许把 mirror/import/export/push 标记为
  unavailable 或 `GitMirrorOutOfSync`；它 **MUST NOT** 阻断 LocalBackend 启动、
  撤销已经成立的 NoteGit commit，或改变 ledger/source-control authority。
  受控 Desktop sidecar 在没有 trusted path 时 **MUST** 携带
  `DEVE_GIT_EXECUTABLE_UNAVAILABLE=1`；core 必须据此禁止普通 `git` 路径搜索。
- Git push 只能发布已映射的 `.git` main mirror HEAD；它 **MUST** fail-closed 于 mirror 未 ready、
  Source Control 不干净、Git worktree 不干净、存在 queued/out-of-sync record、`.notegit`
  tracked 泄漏或 Git HEAD 未映射到最新 NoteGit/ngit commit。
- repair action schema 只能用于诊断、人工修复指引和显式 retry；**MUST NOT** 被 Web、后台任务或 Command Palette 解释为自动 Git 写入授权。
- 自动后台执行、可点击 repair UI 与 Web 后端直接执行 Git 写入 **MUST** 作为独立设计批次处理，不能从只读 status/review surface 隐式升级。
- Proxy / plugin-host node role 摘要不得展示 legacy bridge mode；如果需要描述 Source Control，
  必须展示 NoteGit/ngit authority 与 delegated/readonly 状态。

### 2.3.2 Remote Projection Transport Contract {#remote-projection-transport-contract}

Remote Projection Transport 是 WebDAV/S3 的 host transport boundary，只提供两种语义分离的能力：

```text
Projection Workspace -> push adapter -> remote provider
remote provider -> ordered source acquisition -> project-owned bounded sink
```

- push 只能上传当前 Markdown Projection Workspace 文件集合，不上传 `.notegit/`、`.git/`、Ledger、staging、snapshot、Remote Import artifact 或 runtime state。
- source acquisition 只负责 locator/profile admission、确定性列举与逐文件 streaming；它不拥有 Remote Import session、Ledger、Projection Workspace、External Changes 或 apply 决策。
- push 与 source acquisition 可以复用 provider、credential/profile、HTTP/signing 基础设施，但必须使用语义分离的 typed interface。locator/profile 必须继续满足 ADR 0008 的 exact host-local binding，禁止 ambient credential fallback 到任意 custom host。
- Web surface 只发送 typed intent；provider I/O、路径归一化、预算 admission 与失败分类全部属于 backend/host infra。

#### Current Pull Transition Anchor {#remote-projection-transport}

当前代码仍保留 `webdav:pull` / `s3:pull` 覆盖 workspace、再进入 External Changes 的未发布实现。它只是 B4 前用于保持现有代码 `plan_ref` 可追踪的 release-blocking drift；不是批准目标、兼容 epoch 或可继续扩展的路线。B4 必须连同 workspace apply/rollback continuation、External Changes scan bridge 与旧命令一次性删除。

### 2.3.3 Remote Import Diff Contract {#remote-import-diff-contract}

- 每个 `RemoteImportCandidateRevision` 都绑定 immutable source manifest/blobs 与生成该 revision 时的 exact Ledger head/branch/locator/ignore snapshot；初始 revision 使用 Prepare 基线，Refresh 可按下述受限规则生成新 revision。diff、label、blocker 和 `entry_id` 均由 backend 生成。
- change kind 固定为 `Added | Modified | Unchanged`。远端缺失文件不产生 `Delete`；change kind 与 typed blocker 正交。
- 首版为 whole-session review/apply：不提供 checkbox、逐文件选择、逐文件 apply/discard。任一 blocker 禁用整个 session Apply。
- pending/staged overlap、head/branch/locator/ignore drift、digest mismatch、scope/session/revision mismatch 必须由 backend fail-closed；前端不得重新推导 overlap、stale 或可写性。
- Diff 请求只接受 opaque strong `entry_id`，返回 backend-generated display label 与 typed diff；不得暴露 locator、provider/host path、blob path、digest、credential 或原始失败 detail。
- Refresh 只能从已封存 blobs 重算 candidate。它可以在 `RepoId`、branch、source snapshot 与 locator/profile binding 均未变化且 digests 全部通过时，把新 revision 重新绑定到当前 Ledger head 与当前 ignore snapshot；这正是 head/ignore drift 后 `Stale -> Ready` 的唯一恢复路径。locator/profile、branch、RepoId membership 或 source digest 变化不可重绑，session 保持 Stale 或进入 Failed；需要新远端内容时必须先 Discard，再重新 Prepare。
- Remote Import review/runtime 不复用 Source Control 或 External Changes controller、state、notice 或 authority；只允许复用无状态 diff/render primitive。

### 2.4 Diff Identity Model

- 文本 diff 身份：
  - `DocId`
  - UTF-16 position space
- 结构 diff 身份：
  - `NodeId`
  - `DocId`
- UI 中的 `path` 只是 selector/display，不得作为长期 merge identity。

### 2.5 Typed Diff Projection Contract {#typed-diff-projection-contract}

Diff 算法、replacement pairing、word range、hunk 与 fold 都属于 Core
projection computation；Web 只渲染后端生成的不可变 typed projection，不得
重新运行 Patience/Myers、推断 hunk/fold，或在失败时回退客户端算法。

`DiffProjection` 必须只保存一次 `base_content` / `target_content`，并通过
`DiffCellProjection.byte_range` 引用正文，避免为每个 row 复制行文本。投影至少携带：

- opaque `projection_id`、`DiffAlgorithm::{Myers, PatienceMyers}`、计算耗时；
- canonical `DiffRowProjection`，其中 `row_id` 从 0 开始、左右 cell 可独立为空；
- cell 的可选 1-based 行号、相对正文的 UTF-8 byte 半开区间、相对 cell 文本的
  UTF-16 word highlight 半开区间与 `Context/Add/Delete/Empty` kind；
- canonical row 半开区间的 hunk、old/new 1-based 行范围；
- `context_lines = 3/5/8` 的后端 fold ranges，包含稳定 fold id；
- added/deleted 统计。

输入必须按整个文档计算，不得重新引入固定 300 行分块的语义边界。base 与 target
UTF-8 合计不得超过 8 MiB，总行数不得超过 100,000；最终协议编码不得超过现有
16 MiB WS frame。超限或计算失败必须使用 `13_i18n` 的结构化 `DIFF_*` 错误，
错误详情不得包含正文。Core 计算使用单次 5 秒 wall-clock budget；底层算法 deadline
只能用于终止病理输入，deadline 到达后的近似结果 **MUST NOT** 发布。行数上限必须在
分配完整行索引前无分配计数并 fail-fast。

Commit compare 的列表请求只返回 `CommitFileDiffSummary` 元数据和精确
`CommitFileDiffTarget`。用户选择文件后，服务端必须重新验证 commit A/B 与 target
的 `doc_id/path/previous_path/status` 完全一致，再重建该文件并计算 projection；任一
字段不匹配必须 fail-closed。Core/HTTP/plugin 的原始 `CommitFileDiff` 查询可以继续
作为非浏览器内部接口，但 Web WS 不得一次传输整个提交的所有正文。

可编辑 merge draft 只能发送 base/draft 内容与递增 revision 作为只读计算 intent；
projection 不授予写入 authority。服务端必须在 session/scope/revision 再验证后发布
结果，旧 revision 结果静默丢弃。repo/branch/scope 切换或连接关闭必须取消该 session
的活跃计算。同一 session 的计算必须串行，最新请求最多保留一个等待槽；大型 typed
projection 必须通过独立的一槽有界出站通道交给 WS sender，不得进入通用 must-deliver
无界等待任务。

`MergeConflict.result_content` 是后端生成的安全初始 merge draft，不得使用共同祖先正文
冒充合并结果。Web 可以编辑该 draft，但不得自行推导 AcceptBoth 内容。

## 3. State Machines

### 3.1 External Changes Lifecycle

```text
ProjectionWorkspaceChangeDetected
  -> PendingFsEntry
  -> ExternalChangeStaged
  -> AppliedToLedger
  -> ConfirmedLedgerChange
  -> Cleared
```

旁路：

```text
PendingFsEntry -> Discarded
PendingFsEntry -> Conflict
ExternalChangeStaged -> ExternalChangeUnstaged
PendingFsEntry + ConfirmedLedgerChange(same doc) -> OverlapBlocked
```

约束：

- Watcher 检测到的变更 **MUST NOT** 直接写入 ledger。
- External Changes 的 `Stage` 是 repo-scoped side-table 迁移，不是 UI 样式变化，也不是 Source Control commit anchor 的 include/exclude 模型。
- `Unstage` 必须先捕获完整 staged entry，再在单个 write transaction 内按当前 target 重新解析并与该 entry exact-compare；只有比较成功后才能同时删除 staged row/index 并写回 pending row/index。若 watcher 已在同路径写入语义不同的较新 pending row，必须保留该证据并整笔 fail-closed；语义相同则保持现有 pending row 字节不变。目标消失、被替换或任一第二步失败时整笔回滚。
- `Stage` **MUST** 把 pending 检测到的 `content_hash` 固化到 staged entry。对非 delete target，
  `Apply to Ledger` preflight **MUST** 重新读取 workspace 内容并比较该 hash；不一致时保留 staging、
  不追加任何 ledger fact，并要求重新 scan/stage。
- 一个 stage batch 的 pending/staged rows 与 DocId indexes **MUST** 原子迁移；Apply preflight 捕获的
  内容快照是该批唯一写入输入，所有 target facts、projection/index 更新与本次 staged snapshot 的
  exact consumption **MUST** 共享一个 write transaction。hash 校验后不得二次读取 workspace，
  最终事务必须用 preflight 前的 ledger head 做 compare-and-fail gate；head 漂移时不得使用旧
  confirmed/content base。不得留下批次前缀的 ledger facts，也不得清除本次 snapshot 之外的新 staging。
- `Discard` 的语义是恢复 workspace 到当前规范 projection。
- 普通 `Stage` **MUST** fail-closed 于 `has_conflict=true` 的 pending entry；
  只有显式 `ResolveConflict(KeepFs)` flow 可以通过 resolved-stage 路径清除 conflict 标记并移入 staged。
- `Apply to Ledger` / `确认外部修改` **MUST** 使用 core/server 写入路径把 staged external changes
  转换为 ledger facts；成功后清空 External Changes staging，并让 Source Control 从 commit anchor 到
  ledger head 派生 `ConfirmedLedgerChange`。
- Apply 事务成功后，Core **MUST** 返回 typed
  `ExternalApplyReceipt { repo_id, authority_head: GlobalSeq, affected_docs, applied_target_count }`；HTTP 直接返回
  receipt，WS request 必须带 `request_id` 并返回 `ExternalApplyAck`。server 对一次 Apply 只发布一条
  `ProjectionRecoveryRequired`，由 recovery plan 指定受影响文档以及 DocList/tree、Source Control、
  External Changes 是否刷新；不得逐 fact 广播 `ExternalApplyContentFact` / `NewOp`，也不得在 commit
  后查询 confirmed list 才决定 Apply 是否成功。事务失败不得发布前缀事件，Web 不得从 HTTP
  response、workspace 正文或路径重算 content ops。
- Apply 的目录扫描、workspace 读取、内容重建与 patch 计算必须在 repo permit 外产生 opaque prepared
  input；进入 permit 后必须重新绑定 exact `RepoId`，并 exact-compare ledger head、staged rows 与 path
  identity，再由单个 authority transaction 消费。任一比较失败都保留原 staging 并要求重新 preflight。
- `Apply to Ledger` **MUST NOT** 创建 commit anchor、更新 history、写 Git main mirror queue 或把 staged
  external changes 伪装成 Source Control commit。
- 当 external change 与 confirmed ledger dirty 指向同一 `DocId`，或缺失 `DocId` 时指向同一 canonical
  path，普通 stage/apply **MUST** 禁用；UI 和 API 都必须保留 fail-closed 语义，不得由前端自行覆盖。
- Source Control / External Changes read projection **MUST** 将 staged / unstaged external
  entry 与 confirmed ledger dirty 的重叠派生为 typed conflict state（当前为 `ChangeEntry.has_conflict=true`）；
  view 层只能消费该状态，不得重新实现 doc/path/rename overlap 判断。

补充：

- watcher **MUST** 忽略 `.notegit/`
- watcher、启动扫描、目录重扫 **MUST** 统一应用 `.deveignore`；被忽略路径不得生成 pending / staged / ledger diff。
- `pending_fs_ops` **MUST** 表示 External Changes Domain
- `staging` **MUST** 表示用户确认后的 External Changes 过渡域，而非已提交历史或 commit anchor include set

### 3.1.1 Confirmed Ledger Dirty Lifecycle

```text
EditorOrCliLedgerWrite
  -> LedgerCommitted
  -> ConfirmedLedgerChange
  -> CommitAnchorCovered
  -> Clean
```

约束：

- confirmed ledger dirty 只表示“已进 ledger、未被最新 Source Control commit anchor 覆盖”。
- 它不得进入 `pending_fs_ops`、staging、pending overlay 或 watcher 清理流程。
- 首版采用整锚提交：一次 commit 覆盖 latest commit anchor 到当前 ledger head 的全部 confirmed ledger changes。
- confirmed-only commit 在创建 commit anchor 时必须同步 `snapshot_index` / committed snapshot base 到当前 ledger projection；否则后续 Working Directory conflict 检测会把已提交 ledger 内容误判为 ledger divergence。
- 首版不支持逐文件 include/exclude，也不开放 confirmed revert。未来若开放 Revert，必须通过追加反向 ledger facts 完成。
- Source Control **MUST NOT** 对 `ConfirmedLedgerChange` 提供 Discard；撤回已确认 ledger facts 的唯一未来方向是显式 Revert flow，且 Revert 必须追加反向 ledger facts。

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
  -> MergeCheckpointResolved | InitialEqualBaselineEstablished
  -> DiffCalculated
  -> AutoMerged | Conflict
  -> ConfirmedResult
  -> MergeAnchorCommitted
```

约束：

- merge 只允许发生在同一逻辑 repo 内。
- cross-repo merge **MUST** fail-closed。
- merge runtime 只能由本机 local writer 发起；remote mirror / shadow branch 是只读输入，不是并发写者。
- `MergePeer` **MUST** 从 Local Branch 发起；用户选择 peer force-mirror / shadow branch 作为只读 source。
- Remote Branch UI / scope **MUST NOT** 发起 `MergePeer`、`ResolveMergeConflict` 或任何会产生 local ledger facts 的 merge apply；应禁用入口或 fail-closed。
- `MergePeer` 产出的任何写入都必须重新进入本机 repo-scoped writer gate，并作为新的 Local Branch ledger facts 提交。
- local ledger 与单-source shadow 的物理 `PeerId` 集合按设计互斥；merge **MUST NOT** 再用两边 VersionVector 的 peer-id 交集伪造 LCA。
- `(source_peer_id, DocId)` 已存在有效 `MergeBaseCheckpoint` 时，`Base` 必须由 source shadow 在 checkpoint 的 source waterline 重建，并用 `source_state_hash` 校验该历史仍可证明；checkpoint 的 local anchor 必须仍存在于本地连续历史，以证明本机已经处理过该 source 状态。
- 首次不存在 checkpoint 时，只有 local 与 remote 当前内容逐字节相等才允许自动追加 `MergeAnchor(resolution=establish_equal)` 建立基线；首次已经分叉必须返回 `merge_base_missing` 或等价结构化错误，要求显式 baseline/import 决策，不得以空内容、相似度或路径猜测共同祖先。
- merge preflight 必须是 core 评估产生的 opaque evidence，且 local/source 目标 DocId 都必须由对应事实证明存在，不能把缺失文档折叠为空字符串。writer 在同一 shadow guard 下必须重新执行 evaluation 并逐字段比对 local waterline、source waterline、checkpoint anchor、base/result evidence；任一伪造或漂移都必须 fail-closed 并要求刷新。
- auto merge 与所有 conflict resolution（包括内容不变的 AcceptCurrent）都必须追加本地 `MergeAnchor`；内容 fact、anchor、peer sequence/index 与 checkpoint 更新必须在同一事务提交。
- `MergeAnchor` 不改变 Markdown/tree projection，但必须进入 peer range、完整事实 snapshot、v2 JSON archive 与审计输出；projection fold 必须显式忽略其内容效果。

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
- `CommitSourceControlChanges`
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
- Remote Branch readonly gate **MUST** 禁止 stage、discard、unstage、commit、merge apply、conflict resolve 与任何 plugin-host writer；允许的 Source Control 能力仅限 diff / history / graph / inspect 这类只读派生查询。
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
  - `pending/staged/confirmed entries`
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

`CommitSourceControlChanges` **MUST** 以如下路径完成：

1. 校验 commit message 非空，并解析当前 local repo writer scope。
2. 读取当前 repo 的 ordinary External Changes staging 与 confirmed ledger dirty。
3. ordinary External Changes staging **MUST NOT** 被普通 Source Control commit 直接消费；用户必须先通过
   `ApplyToLedger` 将这些 staged external entries 写入 ledger facts。
4. 若 staged entries 全部来自显式 resolved-conflict flow，commit runtime **MAY** 在同一 writer gate
   下先把 resolved result 写入 ledger facts；该例外不得扩展到普通外部文件 staging。
5. 对 confirmed ledger dirty 创建 commit record，锚定最终 `ledger_seq`。
6. 清理已被 resolved-conflict flow 消费的 staging；ordinary External Changes staging 必须保留给
   External Changes runtime。
7. 重建或增量更新 projection / committed snapshot base。

原子性约束：步骤 5 对应的 commit payload/order anchor 与步骤 7 的全部 committed snapshot baseline 必须在同一个 redb write transaction 内提交；事务必须 exact-compare 预检得到的 ledger head，head 漂移或任一文档 snapshot、commit/order 写入失败时整笔回滚。resolved-conflict patch 与 commit snapshot 的只读准备必须发生在 repo permit 外，permit 内只做 exact revalidation 与提交。snapshot 必须在该事务视图内逐文档读取 facts、重建并立即写入，不得同时缓存全部 dirty 文档的完整内容。Git mirror queue 必须在该事务 commit 之后执行，并携带提交时的 expected `RepoId`；执行时名称若已重绑到其他 repo 必须拒绝。queue 失败只产生可恢复诊断，不得撤销 NoteGit commit。

规则：

- confirmed ledger dirty 非空，或全部 staged entries 为 resolved-conflict flow 且能成功生成 confirmed
  ledger dirty 时，才允许 commit。
- ordinary External Changes staging 不构成 Source Control commit 输入；若只存在 ordinary external staged
  entries，必须返回 `SC_NOTHING_TO_COMMIT` 或等价 no-eligible-changes 结构化错误，并保持 staging 不变。
- eligible confirmed dirty 为空时必须返回 `SC_NOTHING_TO_COMMIT`。
- confirmed-only commit **MUST NOT** 重放、复制或改写已存在的 ledger facts。

### 5.4 Merge Contract

- `Base` = 最新有效 `MergeBaseCheckpoint` 指向的 source shadow state；首次仅允许 equal-state anchor 建立。上次本地 merge result 不得伪装成远端也拥有的共同状态。
- `Left` = local confirmed state。
- `Right` = remote mirror confirmed state。
- auto-merge 仅允许在差异域不重叠时发生。
- conflict result 必须显式进入 resolution flow，不得静默写盘。

补充：

- `MergePeer` 的最终写入只能进入 Local Branch。
- Remote mirror 自身不得因 merge 流程被直接改写。
- merge 不是远端并发编辑协议；它是本机对 remote mirror snapshot 的显式读入、计算与本地提交。
- merge result 若需要用户选择，结果必须在确认后才允许落 ledger。
- 一个 source 的 checkpoint 按 `(source_peer_id, DocId)` 独立维护；合并其他 peer 产生的本地变化可以进入当前 Local，但不得覆盖该 source 的上次共同基线。

## 6. Failure Modes

错误码清单以 `13_i18n.md#i18n-error-code-catalog` 为唯一权威；本节只列失败域。

### 6.1 Pending / Staging Failures

- watcher overflow
- ambiguous path target
- missing staged workspace file
- staged workspace content hash changed after stage
- staged entry doc identity missing

### 6.2 Commit Failures

- workspace file not readable
- structure diff cannot resolve stable identity
- ledger append rejected
- projection writeback failed
- confirmed ledger diff cannot be derived from commit anchor

### 6.3 Merge Failures

- no common ancestor
- merge checkpoint missing / dangling / hash mismatch
- local/source waterline changed after merge preflight
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
- 把 confirmed ledger dirty 回灌成 `pending_fs_ops` 或 staging。

## 9. Runtime Boundary

### 9.1 Authority / Diff Core

职责：

- pending/staging/commit indexes
- confirmed ledger dirty derivation
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
- WebSocket Source Control mutation 不得把这个 HTTP lease 当作当前连接的 writer authority：它必须重新
  验证当前 `WsSession` 的 authenticated peer、local repo、sync handshake、registered writer 与
  `scope_nonce` 全部精确匹配。验证成功后可以为同一 session 续租 HTTP grant；因此仅 HTTP grant
  自然过期不得让仍然活跃且精确绑定的 WS writer 永久进入 stale scope。没有 live writer proof 的
  HTTP 请求仍必须在 lease 过期后 fail-closed，前端不得自行续租或伪造 writer identity。
- Source Control commit、resolved-conflict commit、commit-and-push 的 authority 部分与 merge result
  必须进入 `03_storage/authority#repo-mutation-publication-gate`。Commit anchor lifecycle event 与
  projection recovery 必须按 gate 内 enqueue 顺序发布；Git mirror queue、push 与 HTTP response
  delivery 保持在 gate 外，失败不得回滚已经成立的 NoteGit commit。

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
- independent Remote Import request/revision binding and typed diff projection

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

workspace diff 与 confirmed ledger diff 必须分离；core manager、CLI proxy 与 `use_core` 回调不得共享隐式 source-control 状态。

Remote Import 必须收敛为独立 `remote_import_runtime` 与 `remote_import_client`；Source Control / External Changes 只允许复用无状态 diff primitive，不得承载 Remote Import session 或 apply authority。

## 本章相关命令

- `P2P: Merge Peer`
- `remote_projection.webdav.push`
- `remote_projection.s3.push`
- `remote_import.open`
- `remote_import.refresh`
- `remote_import.apply`
- `remote_import.discard`

## 本章相关配置

- `diff.merge_strategy`: `manual` | `auto`
- Remote projection transport credentials are runtime/admission inputs, not Source Control authority.

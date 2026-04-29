# 07_diff_logic.md - Diff 与版本体验篇

本章描述工作区变更、stage、commit、diff、history 与 merge 的用户体验。

## 功能目标

- 用户应能看清当前有哪些工作区变化。
- 用户应能明确区分 unstaged、staged、committed。
- 冲突与只读场景不能伪装成正常提交。

## Operation 示例

- Source Control commit 原子操作示例见 `docs/features/operations/sc_commit.md`。
- 该示例将 commit 拆为聚焦输入框、输入 message、提交 commit、接收结果四个 user operations。
- Stage / Unstage 原子操作示例见 `docs/features/operations/sc_stage_unstage.md`。
- 该示例将 stage / unstage 拆为发起操作与接收 ack 两段响应。
- Discard File 原子操作示例见 `docs/features/operations/sc_discard_file.md`。
- 该示例将单文件 discard 拆为发送 `DiscardFile` 与接收 `DiscardAck` 两段响应。
- Discard Pending 原子操作示例见 `docs/features/operations/sc_discard_pending.md`。
- 该示例将 repo 级 discard pending 拆为请求 reset-to-projection 与接收 ack 两段响应。
- Resolve Conflict 原子操作示例见 `docs/features/operations/sc_resolve_conflict.md`。
- 该示例将 conflict resolution 拆为 `KeepFs`、`KeepLedger`、接收 `ConflictResolved` 三段响应。
- Merge Peer 原子操作示例见 `docs/features/operations/sc_merge_peer.md`。
- 该示例将 peer merge 拆为选择 peer、发送 `MergePeer`、接收 `MergeComplete`、接收 merge conflict 四段响应。
- Merge Runtime 原子操作示例见 `docs/features/operations/sc_merge_runtime.md`。
- 该示例将 runtime 控件拆为刷新 merge 状态、切换 sync mode、请求 pending ops、确认 merge、接收 runtime status 五段。
- CommitAndPush 原子操作示例见 `docs/features/operations/sc_commit_and_push.md`。
- 该示例将发布型提交拆为聚焦输入框、输入 message、发送 `CommitAndPush`、接收 `CommitAck` 四段。
- Commit History / Commit Diff 原子操作示例见 `docs/features/operations/sc_history_commit_diff.md`。
- 该示例将只读查询链拆为请求 history、接收 history、请求 commit diff、接收 diff result 四段。

## 功能项

### 1. Working Changes

- 外部文件变化或本地工作区偏差应进入变更列表。
- 用户可以看到文件级变化状态。

### 2. Stage / Unstage

- 用户可以把候选变化移入或移出 staged 区域。
- staged 与 unstaged 必须可见区分。

### 3. Diff / History / Graph

- 用户可以打开 diff 查看变更内容。
- 用户可以查看 commit history / graph。
- 这些视图必须与当前 repo scope 一致。
- 当前 graph 的数据面是只读 projection：`deve graph` 与受保护 HTTP query `GET /api/repo/graph` 输出同一类 `GraphProjection` JSON，不写 ledger、workspace、search index 或 source-control state。
- Web 当前只提供最小 Graph panel scaffold：展示 repo-scoped nodes / edges / unresolved counts，以及 loading / failed / empty / local-only fallback。它不执行布局计算，不引入 d3/Pixi，不写任何 authority。
- 当前阶段不承诺高性能 Web graph renderer；Canvas / d3-force / Pixi.js 渲染仍属 future。

### 4. Merge / Conflict

- 冲突必须以显式方式显示。
- 只读或 spectator 场景下不能假装支持 commit/merge 写入。

### 5. Git Mirror Repair UI Boundary

当前 Web 只提供 `Git: Import Changes`、`Git: Push Mirror` 与 `Git: Repair Mirror`
的 CLI-only notice。Git mirror repair 的可点击 UI 属于下一阶段能力，但必须先满足以下边界：

- UI 第一阶段只能展示 `repair_action[...]` / `repair_guidance[...]` 的只读解释与 copyable CLI command，不得直接执行 Git。
- 真实 record-level review 数据源是受保护 HTTP 只读查询 `GET /api/sc/git-mirror/repair-review`；它只读取 server-side `git_mirror_commits` 与 core repair-action schema，不运行 Git，不写 `.git`/`.notegit`，Web 不解析 CLI 输出。
- Source Control repair notice 当前会消费该 endpoint；失败或无 record 时继续显示 CLI-only fallback。
- Repair review 当前支持多条 out-of-sync record 展示，并显式区分 loading、load failed 与 empty record fallback 状态；这些状态只影响展示，不授予 Web repair 写权限。
- Current decision (2026-04-29)：当前批次不实现 executable Web repair UI，也不新增 Web 后端 Git writer；Git import/push/repair 写操作继续只允许通过显式 CLI surface 触发。
- 若后续进入可执行 UI，必须有明确 manual confirmation，且确认内容包含 repo、repair action code、subject、retry command 与 `.notegit` authority 提醒。
- 任何可执行 repair flow 都必须 fail-closed 于 remote/spectator scope、未绑定 repo、writer not ready、dirty Deve Source Control、dirty Git worktree、`.notegit` Git tracking leak 与 stale scope nonce。
- 后台自动 Git writer 不是该 UI 的一部分；`.git` 仍只是 projection mirror，`.notegit` / ledger source-control state 仍是 authority。

## 非目标

- 当前阶段不支持跨 repo 自动 merge。
- 当前阶段不允许 remote spectator 直接提交远端写入。
- 当前阶段不实现 Web 后端直接 Git import/push/repair，也不实现后台自动 Git mirror repair。

## Chrome MCP 验收实例

### DIFF-FEAT-01: Changes -> Stage -> Unstage

前置条件：

- 当前 repo 有至少一个工作区变化。

步骤：

1. 打开 Source Control。
2. 观察 `Changes` 列表。
3. 执行 `Stage`。
4. 再执行 `Unstage`。

期望结果：

- 条目能在 `Changes` 与 `Staged Changes` 之间移动。
- 不出现点击无效或状态错位。

### DIFF-FEAT-02: 打开 Diff 与 History

前置条件：

- 当前 repo 存在变更和历史提交。

步骤：

1. 点击某个 change 打开 diff。
2. 切到 history / graph。
3. 选择一条提交查看详情。

期望结果：

- diff 正常显示。
- history / graph 与当前 repo 一致。

### DIFF-FEAT-03: 只读分支写入边界

前置条件：

- 切换到 remote / spectator 分支。

步骤：

1. 打开 Source Control。
2. 尝试执行 stage、commit 或其它写操作。

期望结果：

- 页面明确显示只读或不可写。
- 不会假装提交成功。

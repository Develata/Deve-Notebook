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

### 4. Merge / Conflict

- 冲突必须以显式方式显示。
- 只读或 spectator 场景下不能假装支持 commit/merge 写入。

## 非目标

- 当前阶段不支持跨 repo 自动 merge。
- 当前阶段不允许 remote spectator 直接提交远端写入。

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

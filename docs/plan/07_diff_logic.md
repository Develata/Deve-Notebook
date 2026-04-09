# 07_diff_logic.md - Diff 工程蓝图

本章只定义 diff、stage、commit、merge 的实现合同，不描述用户交互文案。功能语义见 [../features/07_diff_logic.md](../features/07_diff_logic.md)，自动化验收见 [../acceptance-cases/04_diff.md](../acceptance-cases/04_diff.md)。

## 1. 目标

- diff 只在同一逻辑 repo 内成立。
- working directory、staging、commit history 是三层不同状态，不得混用。
- 文本差异与结构差异必须分域建模。

## 2. 权威实体

- `WorkingChanges`
  - watcher 发现的未确认工作区偏差。
- `StagedChanges`
  - 已明确确认、待提交的候选集合。
- `CommittedFacts`
  - 已追加进 ledger 的 content/structure facts。
- `CommitAnchor`
  - 指向特定 ledger seq 的版本锚点。

## 3. 两个 Diff 域

### 3.1 Working Directory Domain

- 比较 `Vault` 与当前 canonical projection。
- 结果进入 `pending_fs_ops`，不进入 authority。

### 3.2 Commit Domain

- 比较 staged 内容与 ledger anchored base。
- 结果转为 `Content Facts` / `Structure Facts` 并进入 ledger。

## 4. Merge 合同

- merge 只允许在同一 `RepoId` 下进行。
- 文本冲突基于 content diff。
- rename/move/create/delete 冲突必须基于 structure facts，而不是 path 字符串猜测。

## 5. 状态机

- `Clean`
- `WorkingChanged`
- `Staged`
- `Committing`
- `Committed`
- `Conflict`

### 转换规则

- `WatcherChange -> WorkingChanged`
- `Stage -> Staged`
- `CommitStart -> Committing`
- `LedgerAppendOk -> Committed`
- `MergeConflictDetected -> Conflict`

## 6. 文本与结构分工

- Myers / UTF-16 index 只负责文本差异表达。
- create / rename / move / delete 必须作为结构事实表达。
- diff view 可以统一展示，但底层 authority 必须保持分层。

## 7. 失败合同

- 冲突出现时，系统必须进入显式 `Conflict` 状态。
- stage/commit 失败不得伪装为已提交。
- remote spectator/source-control readonly 场景下不得允许 commit 成功假象。

## 8. 禁止事项

- 禁止跨 repo 自动 merge。
- 禁止 watcher 变更直接视为 committed。
- 禁止把 rename/move 仅当作 path string diff。
- 禁止 UI diff 视图直接充当 authority merge 结果。

## 9. 代码边界

- `crates/core/src/source_control/`
  - commit/diff/history/merge authority。
- `crates/core/src/ledger/`
  - content/structure facts append。
- `apps/cli/src/server/handlers/source_control/`
  - source control handler/runtime glue。
- `apps/web/src/components/diff_view/`
  - 只负责展示，不负责 authority merge 决策。

# sc_history_commit_diff.md - Commit History / Commit Diff 查询示例

## Metadata

- `Flow ID`: `flow.sc.history-commit-diff`
- `Domain`: `source-control`
- `Related Feature Chapters`: `docs/features/07_diff_logic.md`
- `Related Acceptance Cases`: `DIFF-FEAT-02`

## Operations

### `op.sc.history.request`
- `Name`: `Request Commit History`
- `Surface`: `source-control-panel`
- `Trigger`: 用户打开 history / graph
- `Immediate Result`: 发送 `ClientMessage::GetCommitHistory`

### `op.sc.history.receive`
- `Name`: `Receive Commit History`
- `Surface`: `source-control-panel`
- `Trigger`: 服务端返回 `ServerMessage::CommitHistory`
- `Immediate Result`: 更新当前 repo 的 commit history 列表

### `op.sc.commit-diff.request`
- `Name`: `Request Commit Diff`
- `Surface`: `source-control-panel`
- `Trigger`: 用户选择某条提交查看详情
- `Immediate Result`: 发送 `ClientMessage::GetCommitDiff`

### `op.sc.commit-diff.receive`
- `Name`: `Receive Commit Diff Result`
- `Surface`: `source-control-panel`
- `Trigger`: 服务端返回 `ServerMessage::CommitDiffResult`
- `Immediate Result`: 更新当前 commit diff 视图

## Notes

- 这条 flow 只覆盖 commit history / commit diff，只读查询，不重复 doc diff。

# sc_commit_and_push.md - CommitAndPush / 发布入口示例

## Metadata

- `Flow ID`: `flow.sc.commit-and-push`
- `Domain`: `source-control`
- `Related Feature Chapters`: `docs/features/07_diff_logic.md`, `docs/plan/12_commands.md`
- `Related Acceptance Cases`: `DIFF-FEAT-02`

## Operations

### `op.sc.commit-publish.focus-input`

- `Name`: `Focus Publish Commit Input`
- `Surface`: `source-control-panel`
- `Trigger`: 用户准备执行发布型提交
- `Preconditions`: 当前 repo scope 已绑定
- `Immediate Result`: 进入 commit draft 输入状态
- `Application Entry`: `apps/web/src/components/sidebar/source_control/commit.rs`

### `op.sc.commit-publish.type-message`

- `Name`: `Type Publish Commit Message`
- `Surface`: `source-control-panel`
- `Trigger`: 用户输入提交说明
- `Preconditions`: commit input 已聚焦
- `Immediate Result`: 更新 draft message
- `Application Entry`: `apps/web/src/components/sidebar/source_control/commit_message_box.rs`

### `op.sc.commit-publish.submit`

- `Name`: `Submit CommitAndPush`
- `Surface`: `source-control-panel`
- `Trigger`: 用户点击 `Commit & Push`
- `Preconditions`: write gate 未阻塞，当前是 local repo scope
- `Immediate Result`: 发送 `ClientMessage::CommitAndPush`
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks_sc/write/commit.rs`, `apps/cli/src/server/ws/route/source_control.rs`, `apps/cli/src/server/handlers/source_control/commits.rs`

### `op.sc.commit-publish.receive-result`

- `Name`: `Receive Publish Commit Ack`
- `Surface`: `source-control-panel`
- `Trigger`: 服务端返回 `CommitAck`
- `Preconditions`: `CommitAndPush` 请求已成功进入处理链
- `Immediate Result`: 刷新 changes / commit history，并清空当前 notice
- `Application Entry`: `apps/web/src/hooks/use_core/effects_sc/dispatch_acks.rs`

## Notes

- 当前示例只覆盖代码里已存在的 `CommitAndPush` 入口。
- 它目前仍以 `CommitAck` 为主要完成信号，不额外建模独立 `SyncPush` 用户结果流。

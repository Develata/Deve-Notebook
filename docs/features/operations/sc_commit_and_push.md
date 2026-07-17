# sc_commit_and_push.md - CommitAndPush / 发布入口示例

## Metadata

- `Flow ID`: `flow.sc.commit-and-push`
- `Domain`: `source-control`
- `Related Feature Chapters`: `docs/features/07_diff_logic.md`, `docs/plan/14_commands.md`
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

- `Name`: `Show CommitAndPush CLI-only Notice`
- `Surface`: `source-control-panel`
- `Trigger`: 用户点击 `Commit & Push`
- `Preconditions`: write gate 未阻塞，当前是 local repo scope
- `Immediate Result`: 展示 Git push CLI-only notice；Web 不发送 Git writer intent
- `Application Entry`: `apps/web/src/components/sidebar/source_control/commit_controller.rs`

## Notes

- `CommitAndPush` 不是 WS v2 wire surface；UI callback 只更新本地 notice，不调用 WebSocket client。
- 正常发布流程是先执行 Web `Commit` 或 CLI `deve sc commit`，再通过显式 CLI `deve ngit push` 发布 Git mirror。

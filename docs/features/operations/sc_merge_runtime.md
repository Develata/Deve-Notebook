# sc_merge_runtime.md - Merge Runtime 控制流示例

## Metadata

- `Flow ID`: `flow.sc.merge-runtime`
- `Domain`: `source-control`
- `Related Feature Chapters`: `docs/features/07_diff_logic.md`, `docs/features/05_network.md`
- `Related Acceptance Cases`: `DIFF-005`, `NET-FEAT-03`

## Operations

### `op.sc.merge-runtime.refresh`

- `Name`: `Refresh Merge Runtime`
- `Surface`: `workspace-runtime`
- `Trigger`: 进入当前 local repo scope，或 runtime 需要重新拉取 merge 状态
- `Preconditions`: 当前不是 remote branch scope
- `Immediate Result`: 前端同时请求 `GetSyncMode` 与 `GetPendingOps`
- `Application Entry`: `apps/web/src/hooks/use_core/effects/message_control_runtime_repo/requests.rs`

### `op.sc.merge-runtime.change-mode`

- `Name`: `Change Sync Mode`
- `Surface`: `sync-controls`
- `Trigger`: 用户切换 manual / auto
- `Preconditions`: local repo scope 稳定，write gate 未阻塞
- `Immediate Result`: 发送 `ClientMessage::SetSyncMode`
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks_sync/write.rs`, `apps/cli/src/server/ws/route/merge.rs`, `apps/cli/src/server/handlers/merge/manual.rs`

### `op.sc.merge-runtime.request-pending`

- `Name`: `Request Pending Ops`
- `Surface`: `sync-controls`
- `Trigger`: 用户刷新 pending merge 状态
- `Preconditions`: local repo scope 稳定
- `Immediate Result`: 发送 `ClientMessage::GetPendingOps`
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks_sync/read.rs`, `apps/cli/src/server/ws/route/merge.rs`, `apps/cli/src/server/handlers/merge/manual.rs`

### `op.sc.merge-runtime.confirm`

- `Name`: `Confirm Merge`
- `Surface`: `sync-controls`
- `Trigger`: 用户确认应用当前 pending ops
- `Preconditions`: write gate 未阻塞，pending ops 非空
- `Immediate Result`: 发送 `ClientMessage::ConfirmMerge`
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks_sync/write.rs`, `apps/cli/src/server/ws/route/merge.rs`, `apps/cli/src/server/handlers/merge/manual.rs`

### `op.sc.merge-runtime.receive-status`

- `Name`: `Receive Merge Runtime Status`
- `Surface`: `sync-controls`
- `Trigger`: 服务端返回 `SyncModeStatus`、`PendingOpsInfo` 或 `MergeComplete`
- `Preconditions`: 相关 runtime 请求已发出，或 merge 已完成
- `Immediate Result`: 更新 sync mode、pending count/previews，或清空 pending merge 状态
- `Application Entry`: `apps/web/src/hooks/use_core/effects/message_dispatch_route_projection/sync.rs`, `apps/web/src/hooks/use_core/effects/message_runtime_sync/mod.rs`

## Notes

- 这条 flow 关注 merge runtime 控件本身，不重复 `merge peer`、`discard pending` 或 `resolve conflict`。
- `Refresh Merge Runtime` 是 runtime 侧 operation，不一定总是来自显式按钮点击。
- `Confirm Merge` 的最终成立条件仍回到 sync engine 应用 remote ops 与 ledger/snapshot 持久化。

# sc_discard_pending.md - Discard Pending 操作流示例

## Metadata

- `Flow ID`: `flow.sc.discard-pending`
- `Domain`: `source-control`
- `Related Feature Chapters`: `docs/features/07_diff_logic.md`
- `Related Acceptance Cases`: `DIFF-FEAT-03`

## Operations

### `op.sc.discard.request`

- `Name`: `Request Discard Pending`
- `Surface`: `source-control-panel`
- `Trigger`: 点击 `Discard All`，或执行等价 discard pending 操作
- `Preconditions`: 当前 repo scope 稳定，write gate 未阻塞，存在待丢弃的 pending changes / ops
- `Immediate Result`: 前端发送 `ClientMessage::DiscardPending`
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks_sync/write.rs`, `apps/cli/src/server/ws/route/merge.rs`, `apps/cli/src/server/handlers/merge/manual.rs`

### `op.sc.discard.receive-ack`

- `Name`: `Receive Discard Ack`
- `Surface`: `source-control-panel`
- `Trigger`: 服务端返回 `PendingDiscarded`
- `Preconditions`: `op.sc.discard.request` 已执行
- `Immediate Result`: pending 区被清空，工作区回到当前 projection
- `Application Entry`: `apps/cli/src/server/handlers/merge/manual.rs`, `apps/web/src/hooks/use_core/effects/message_dispatch_sync.rs`, `apps/web/src/hooks/use_core/effects/message_runtime_sync/mod.rs`

## Notes

- 这条 flow 建模的是 repo 级 `discard pending / reset-to-projection`，不是单文件 `DiscardFile`。
- 核心语义是清空 pending 并恢复到 ledger projection，而不是只刷新 UI 列表。

# sc_resolve_conflict.md - Resolve Conflict 操作流示例

## Metadata

- `Flow ID`: `flow.sc.resolve-conflict`
- `Domain`: `source-control`
- `Related Feature Chapters`: `docs/features/07_diff_logic.md`
- `Related Acceptance Cases`: `DIFF-FEAT-03`

## Operations

### `op.sc.conflict.keep-fs`

- `Name`: `Choose KeepFs`
- `Surface`: `source-control-panel`
- `Trigger`: 用户在 conflict 条目上点击 `KeepFs`
- `Preconditions`: 条目 `has_conflict=true`，write gate 未阻塞
- `Immediate Result`: 前端发送 `ClientMessage::ResolveConflict { resolution: KeepFs }`
- `Application Entry`: `apps/web/src/components/sidebar/source_control/change_item_conflict_actions.rs`, `apps/web/src/hooks/use_core/callbacks_sc/write/commit.rs`, `apps/cli/src/server/ws/route/source_control.rs`

### `op.sc.conflict.keep-ledger`

- `Name`: `Choose KeepLedger`
- `Surface`: `source-control-panel`
- `Trigger`: 用户在 conflict 条目上点击 `KeepLedger`
- `Preconditions`: 条目 `has_conflict=true`，write gate 未阻塞
- `Immediate Result`: 前端发送 `ClientMessage::ResolveConflict { resolution: KeepLedger }`
- `Application Entry`: `apps/web/src/components/sidebar/source_control/change_item_conflict_actions.rs`, `apps/web/src/hooks/use_core/callbacks_sc/write/commit.rs`, `apps/cli/src/server/ws/route/source_control.rs`

### `op.sc.conflict.receive-resolved`

- `Name`: `Receive Conflict Resolved`
- `Surface`: `source-control-panel`
- `Trigger`: 服务端返回 `ConflictResolved`
- `Preconditions`: `op.sc.conflict.keep-fs` 或 `op.sc.conflict.keep-ledger` 已执行
- `Immediate Result`: conflict 标记消失并刷新 changes list
- `Application Entry`: `apps/cli/src/server/handlers/source_control/conflict.rs`, `apps/web/src/hooks/use_core/effects_sc/dispatch_acks.rs`

## Notes

- 这条 flow 只建模显式 conflict resolution，不重复普通 discard / stage。
- `KeepFs` 走 resolved-stage 路径，清除 conflict 标记后移入 staged；`KeepLedger` 走 `discard_via_sync_manager`。

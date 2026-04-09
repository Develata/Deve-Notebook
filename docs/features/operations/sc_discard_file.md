# sc_discard_file.md - Discard File 操作流示例

## Metadata

- `Flow ID`: `flow.sc.discard-file`
- `Domain`: `source-control`
- `Related Feature Chapters`: `docs/features/07_diff_logic.md`
- `Related Acceptance Cases`: `DIFF-FEAT-01`, `DIFF-FEAT-03`

## Operations

### `op.sc.discard-file.entry`

- `Name`: `Discard Change Entry`
- `Surface`: `source-control-panel`
- `Trigger`: 点击单个 change 的 `Discard`
- `Preconditions`: 当前条目位于 unstaged / working changes，write gate 未阻塞
- `Immediate Result`: 前端发送 `ClientMessage::DiscardFile`
- `Application Entry`: `apps/web/src/components/sidebar/source_control/change_item_workspace_actions.rs`, `apps/web/src/hooks/use_core/callbacks_sc_write_targets.rs`, `apps/cli/src/server/ws/route/source_control.rs`

### `op.sc.discard-file.receive-ack`

- `Name`: `Receive Discard File Ack`
- `Surface`: `source-control-panel`
- `Trigger`: 服务端返回 `DiscardAck` 与刷新后的 `ChangesList`
- `Preconditions`: `op.sc.discard-file.entry` 已执行
- `Immediate Result`: 目标条目从工作区变更列表消失，并恢复到当前 projection
- `Application Entry`: `apps/cli/src/server/handlers/source_control/discard.rs`, `apps/web/src/hooks/use_core/effects_sc.rs`

## Notes

- 这条 flow 建模的是单文件级 `DiscardFile`，不是 repo 级 `DiscardPending`。
- 核心语义是按 target 恢复 projection 路径，并清理该条目的 pending entry。

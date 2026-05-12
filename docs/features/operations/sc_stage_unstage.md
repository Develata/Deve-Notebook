# sc_stage_unstage.md - Stage / Unstage 操作流示例

## Metadata

- `Flow ID`: `flow.sc.stage-unstage`
- `Domain`: `source-control`
- `Related Feature Chapters`: `docs/features/07_diff_logic.md`
- `Related Acceptance Cases`: `DIFF-FEAT-01`, `DIFF-FEAT-03`

## Operations

### `op.sc.stage.entry`

- `Name`: `Stage Change Entry`
- `Surface`: `source-control-panel`
- `Trigger`: 点击单个 change 的 `Stage`
- `Preconditions`: 当前 change 位于 unstaged 区域，write gate 未阻塞
- `Immediate Result`: 前端发送 `ClientMessage::StageFile`
- `Application Entry`: `apps/web/src/components/sidebar/source_control/change_item.rs`, `apps/web/src/hooks/use_core/callbacks_sc/write/targets.rs`

### `op.sc.stage.receive-ack`

- `Name`: `Receive Stage Ack`
- `Surface`: `source-control-panel`
- `Trigger`: 服务端返回 `StageAck` 或刷新后的 `ChangesList`
- `Preconditions`: `op.sc.stage.entry` 已执行
- `Immediate Result`: 条目从 unstaged 移入 staged
- `Application Entry`: `apps/cli/src/server/handlers/source_control/staging.rs`, `apps/web/src/hooks/use_core/effects_sc.rs`

### `op.sc.unstage.entry`

- `Name`: `Unstage Change Entry`
- `Surface`: `source-control-panel`
- `Trigger`: 点击单个 staged change 的 `Unstage`
- `Preconditions`: 条目当前位于 staged 区域，write gate 未阻塞
- `Immediate Result`: 前端发送 `ClientMessage::UnstageFile`
- `Application Entry`: `apps/web/src/components/sidebar/source_control/change_item.rs`, `apps/web/src/hooks/use_core/callbacks_sc/write/targets.rs`

### `op.sc.unstage.receive-ack`

- `Name`: `Receive Unstage Ack`
- `Surface`: `source-control-panel`
- `Trigger`: 服务端返回 `UnstageAck` 或刷新后的 `ChangesList`
- `Preconditions`: `op.sc.unstage.entry` 已执行
- `Immediate Result`: 条目从 staged 移回 unstaged
- `Application Entry`: `apps/cli/src/server/handlers/source_control/staging.rs`, `apps/web/src/hooks/use_core/effects_sc.rs`

## Notes

- 这条 flow 强调 `pending -> staged -> pending` 的显式迁移，而不是 UI 样式切换。
- `Stage All / Unstage All` 只是同一语义的批量版本，不单独在这轮建模。

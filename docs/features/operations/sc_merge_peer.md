# sc_merge_peer.md - Merge Peer 操作流示例

## Metadata

- `Flow ID`: `flow.sc.merge-peer`
- `Domain`: `source-control`
- `Related Feature Chapters`: `docs/features/07_diff_logic.md`, `docs/features/12_commands.md`
- `Related Acceptance Cases`: `DIFF-002`, `DIFF-003`, `DIFF-004`, `DIFF-005`

## Operations

### `op.sc.merge-peer.choose-target`

- `Name`: `Choose Merge Peer Target`
- `Surface`: `command-palette`
- `Trigger`: 用户在 command palette 里执行 `merge_peer`
- `Preconditions`: 当前 remote peer / branch 已选中
- `Immediate Result`: merge 目标 peer 被解析为当前 active branch
- `Application Entry`: `apps/web/src/components/command_palette/registry.rs`

### `op.sc.merge-peer.request`

- `Name`: `Request MergePeer`
- `Surface`: `workspace-runtime`
- `Trigger`: merge command action 触发
- `Preconditions`: current doc 已选中，local repo scope 稳定，write gate 未阻塞
- `Immediate Result`: 前端发送 `ClientMessage::MergePeer { peer_id, doc_id, scope_nonce }`
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks_sync/write.rs`, `apps/cli/src/server/ws/route/merge.rs`, `apps/cli/src/server/handlers/merge/peer.rs`

### `op.sc.merge-peer.receive-complete`

- `Name`: `Receive MergeComplete`
- `Surface`: `workspace-runtime`
- `Trigger`: 服务端广播 `ServerMessage::MergeComplete`
- `Preconditions`: `op.sc.merge-peer.request` 已发送且 merge 成功
- `Immediate Result`: pending merge state 清理，runtime 显示合并成功
- `Application Entry`: `apps/web/src/hooks/use_core/effects/message_runtime_sync/mod.rs`

### `op.sc.merge-peer.receive-conflict`

- `Name`: `Receive Merge Conflict`
- `Surface`: `diff-view`
- `Trigger`: 服务端返回 `ServerMessage::MergeConflict`
- `Preconditions`: `op.sc.merge-peer.request` 已发送且 remote/local 无法直接合并
- `Immediate Result`: 打开 diff/conflict surface，等待用户后续选择
- `Application Entry`: `apps/cli/src/server/handlers/merge/peer.rs`, `apps/web/src/hooks/use_core/effects_sc/dispatch_lists.rs`, `apps/web/src/components/diff_view/conflict_actions.rs`

## Response Flows

### `op.sc.merge-peer.choose-target`

1. `User Operation`: 用户在 command palette 中执行 `merge_peer`。
2. `Application Response`: registry action 读取当前 active branch，并把 peer id 传入 sync merge callback。
3. `Concrete Modules`:
   - `apps/web/src/components/command_palette/registry.rs`
4. `Core Subsystems`: 无。此步只解析前端 target。

### `op.sc.merge-peer.request`

1. `User Operation`: 用户发起合并当前 peer。
2. `Application Response`: write gate 先检查 repo scope、writer-ready、readonly、current doc；通过后发送 `MergePeer`。
3. `Concrete Modules`:
   - `apps/web/src/hooks/use_core/callbacks_sync/write.rs`
   - `apps/web/src/hooks/use_core/write_gate/logic.rs`
   - `apps/cli/src/server/ws/route/merge.rs`
   - `apps/cli/src/server/handlers/merge/peer.rs`
   - `crates/core/src/ledger/merge/`
   - `crates/core/src/sync/reconcile.rs`
4. `Core Subsystems`:
   - `ledger`
   - `sync`
   - `protocol`

### `op.sc.merge-peer.receive-complete`

1. `User Operation`: 用户观察 merge 成功结果。
2. `Application Response`: runtime 接收 `MergeComplete`，清理 pending merge 状态并显示 merged count。
3. `Concrete Modules`:
   - `apps/web/src/hooks/use_core/effects/message_runtime_sync/mod.rs`
   - `apps/cli/src/server/handlers/merge/peer.rs`
4. `Core Subsystems`:
   - `sync`
   - `protocol`

### `op.sc.merge-peer.receive-conflict`

1. `User Operation`: 用户观察 merge conflict 结果。
2. `Application Response`: 服务端优先发送 typed `MergeConflict`，前端切入 diff/conflict surface；`DocDiff` 仅保留为非权威兼容 fallback。
3. `Concrete Modules`:
   - `apps/cli/src/server/handlers/merge/peer.rs`
   - `apps/cli/src/server/handlers/merge/peer_apply/mod.rs`
   - `apps/web/src/hooks/use_core/effects_sc/dispatch_lists.rs`
   - `apps/web/src/components/diff_view/conflict_actions.rs`
4. `Core Subsystems`:
   - `ledger`
   - `protocol`

#### Conflict Resolution Selector Contract

1. `User Operation`: 用户选择当前、传入或合并结果。
2. `Application Response`: 前端使用 conflict `doc_id` 与当前 `scope_nonce` 发送 `ResolveMergeConflict`；服务端校验 pending conflict 与 scope 后写入所选结果。
3. `Concrete Modules`:
   - `apps/web/src/hooks/use_core/diff_session.rs`
   - `apps/web/src/components/desktop_layout/content.rs`
   - `apps/web/src/components/mobile_layout/content.rs`
   - `apps/cli/src/server/ws/route/merge.rs`
   - `apps/cli/src/server/handlers/merge/peer_resolve.rs`
4. `Core Subsystems`:
   - `ledger`
   - `protocol`

## Notes

- 这条 flow 只覆盖 `merge_peer`，不重复 `ConfirmMerge` 或 `DiscardPending`。
- `merge_peer` 是显式用户动作，符合 `05_network` 中“merge 到 local 必须是显式用户动作”的约束。
- conflict 出现后，UI 只发送 typed `ResolveMergeConflict`；旧 `DocDiff` fallback 不得成为 conflict authority。

# net_sync_transfer.md - repo-scoped sync transfer 示例

## Metadata

- `Flow ID`: `flow.net.sync-transfer`
- `Domain`: `network`
- `Related Feature Chapters`: `docs/features/05_network.md`
- `Related Acceptance Cases`: `NET-FEAT-02`, `NET-FEAT-03`, `NET-FEAT-05`

## Operations

### `op.net.sync.request-missing`

- `Name`: `Request Missing Ops`
- `Surface`: `workspace-runtime`
- `Trigger`: 当前 repo vector 比较后需要增量同步
- `Preconditions`: handshake-ready 且 repo route 有效
- `Immediate Result`: 发送 `ClientMessage::SyncRequest`
- `Application Entry`: `apps/cli/src/server/handlers/sync/transfer.rs`

### `op.net.sync.receive-push`

- `Name`: `Receive SyncPush`
- `Surface`: `workspace-runtime`
- `Trigger`: 服务端返回 `ServerMessage::SyncPush`
- `Preconditions`: 已建立 repo-scoped sync context
- `Immediate Result`: 验证 source 与严格连续的 peer fact range 后，Auto 模式原子应用；Manual 模式先通过累计 payload/fact/encoded-byte resource gate 再暂存，超限或后续合并失败时保持既有队列、shadow/vector 不变
- `Application Entry`: `apps/web/src/editor/sync/route_payload.rs`, `apps/cli/src/server/handlers/sync/transfer.rs`

### `op.net.sync.request-snapshot`

- `Name`: `Request Snapshot Fallback`
- `Surface`: `workspace-runtime`
- `Trigger`: vector gap 过大或增量同步不可继续
- `Preconditions`: 当前 repo 仍处于有效 sync scope
- `Immediate Result`: 发送 `ClientMessage::SyncSnapshotRequest`
- `Application Entry`: `apps/cli/src/server/handlers/sync/snapshot.rs`

### `op.net.sync.receive-snapshot`

- `Name`: `Receive SyncPushSnapshot`
- `Surface`: `workspace-runtime`
- `Trigger`: 服务端返回 `ServerMessage::SyncPushSnapshot`
- `Preconditions`: snapshot fallback 已发起
- `Immediate Result`: 验证 source peer 的 `1..=waterline` 完整连续事实日志；Auto 模式原子替换 shadow，Manual 模式先经累计资源 gate 暂存，确认合并失败时完整恢复待确认队列
- `Application Entry`: `apps/web/src/editor/sync/route_payload.rs`, `apps/cli/src/server/handlers/sync/snapshot.rs`

## Notes

- 这条 flow 是 handshake 之后的 repo-scoped 传输链，不重复 `SyncHello / WriteReady`。
- wire range 是闭区间；缺口恢复属于同一 transfer flow，不新增 UI authority operation。
- 当前示例只覆盖传输与 fallback，不展开 editor projection 或 key-provide 细节。

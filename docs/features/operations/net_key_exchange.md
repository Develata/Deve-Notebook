# net_key_exchange.md - repo-scoped key exchange 示例

## Metadata

- `Flow ID`: `flow.net.key-exchange`
- `Domain`: `network`
- `Related Feature Chapters`: `docs/features/05_network.md`
- `Related Acceptance Cases`: `NET-FEAT-01`, `NET-FEAT-03`

## Operations

### `op.net.key.request`

- `Name`: `Request RepoKey`
- `Surface`: `workspace-runtime`
- `Trigger`: 当前 local repo 已 handshake-ready 且需要解密 repo payload
- `Preconditions`: 当前不是 remote branch scope，连接处于 connected
- `Immediate Result`: 发送 `ClientMessage::RequestKey`
- `Application Entry`: `apps/web/src/editor/request_key.rs`, `apps/cli/src/server/handlers/key_exchange.rs`

### `op.net.key.receive-provide`

- `Name`: `Receive KeyProvide`
- `Surface`: `workspace-runtime`
- `Trigger`: 服务端返回 `ServerMessage::KeyProvide`
- `Preconditions`: 当前 scoped message 仍匹配 repo / branch / scope nonce
- `Immediate Result`: 写入内存态 `RepoKey`，允许后续解密 `SyncPush`
- `Application Entry`: `apps/web/src/editor/sync/dispatch_payload.rs`, `apps/web/src/editor/sync/key.rs`

### `op.net.key.receive-denied`

- `Name`: `Receive KeyDenied`
- `Surface`: `workspace-runtime`
- `Trigger`: 服务端返回 `ServerMessage::KeyDenied`
- `Preconditions`: 当前 scoped message 仍匹配 repo / branch / scope nonce
- `Immediate Result`: 清空内存态 `RepoKey` 并记录告警
- `Application Entry`: `apps/web/src/editor/sync/dispatch_payload.rs`, `apps/cli/src/server/handlers/key_exchange.rs`

## Notes

- 这条 flow 只覆盖浏览器侧 repo key 获取，不重复 `SyncPush` 解密细节。
- `RepoKey` 只允许驻留内存，不进入浏览器持久化存储。

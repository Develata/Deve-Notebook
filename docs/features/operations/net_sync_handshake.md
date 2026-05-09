# net_sync_handshake.md - repo-scoped sync handshake 示例

## Metadata

- `Flow ID`: `flow.net.sync-handshake`
- `Domain`: `network`
- `Related Feature Chapters`: `docs/features/05_network.md`
- `Related Acceptance Cases`: `NET-FEAT-01`, `NET-FEAT-02`, `NET-FEAT-03`

## Operations

### `op.net.sync.resume-runtime`

- `Name`: `Resume Repo Runtime`
- `Surface`: `workspace-runtime`
- `Trigger`: 页面进入当前 repo scope 或重连完成
- `Preconditions`: 已有当前 repo 选择，连接未处于 unauthorized
- `Immediate Result`: 前端开始检查是否应对当前 repo 触发 handshake
- `Application Entry`: `apps/web/src/hooks/use_core/effects/handshake.rs`

### `op.net.sync.send-hello`

- `Name`: `Send SyncHello`
- `Surface`: `workspace-runtime`
- `Trigger`: handshake gate 通过
- `Preconditions`: 当前 repo scope、vector、identity、scope nonce 已就绪
- `Immediate Result`: 发送 `ClientMessage::SyncHello` 与 `RegisterWriter`
- `Application Entry`: `apps/web/src/hooks/use_core/effects/handshake_send.rs`, `apps/web/src/hooks/use_core/effects/handshake_send_delivery.rs`

### `op.net.sync.receive-hello`

- `Name`: `Receive SyncHello Ack`
- `Surface`: `workspace-runtime`
- `Trigger`: 服务端返回 `ServerMessage::SyncHello`
- `Preconditions`: `op.net.sync.send-hello` 已发送
- `Immediate Result`: 当前 repo scope 标记为 handshake-ready，peer/vector 写回本地 runtime
- `Application Entry`: `apps/web/src/hooks/use_core/effects/message_sync.rs`

### `op.net.sync.receive-write-ready`

- `Name`: `Receive WriteReady`
- `Surface`: `workspace-runtime`
- `Trigger`: 服务端返回 `ServerMessage::WriteReady`
- `Preconditions`: repo 已绑定且 writer registration 通过
- `Immediate Result`: 当前 repo 进入 writer-ready，可写 gate 闭合
- `Application Entry`: `apps/web/src/hooks/use_core/effects/message_dispatch_write.rs`

## Response Flows

### `op.net.sync.resume-runtime`

1. `User Operation`: 当前 repo workspace 被恢复或重新进入前台。
2. `Application Response`: handshake cycle 判断连接、repo、scope、branch 与恢复条件是否允许继续。
3. `Concrete Modules`:
   - `apps/web/src/hooks/use_core/effects/handshake.rs`
   - `apps/web/src/hooks/use_core/effects/handshake_cycle.rs`
   - `apps/web/src/hooks/use_core/effects/handshake_state.rs`
4. `Core Subsystems`:
   - `protocol`
   - `sync`

### `op.net.sync.send-hello`

1. `User Operation`: runtime 进入可发起 handshake 的时刻。
2. `Application Response`: 组装 vector，签名握手消息，发送 `SyncHello` 与 `RegisterWriter`。
3. `Concrete Modules`:
   - `apps/web/src/hooks/use_core/effects/handshake_send.rs`
   - `apps/web/src/hooks/use_core/effects/handshake_send_delivery.rs`
   - `apps/cli/src/server/handlers/sync/hello/mod.rs`
   - `apps/cli/src/server/handlers/sync/writer/mod.rs`
4. `Core Subsystems`:
   - `protocol`
   - `sync`

### `op.net.sync.receive-hello`

1. `User Operation`: runtime 收到服务端 handshake ack。
2. `Application Response`: 校验 repo/scope 是否仍匹配，匹配后设置 `handshake_ready` 并更新 peer/vector。
3. `Concrete Modules`:
   - `apps/web/src/hooks/use_core/effects/message_sync.rs`
   - `apps/cli/src/server/handlers/sync/hello/mod.rs`
   - `apps/cli/src/server/handlers/sync/hello/scope.rs`
4. `Core Subsystems`:
   - `protocol`
   - `sync`

### `op.net.sync.receive-write-ready`

1. `User Operation`: runtime 收到 writer-ready。
2. `Application Response`: 校验 repo/branch/scope 仍匹配，匹配后标记当前 repo 可写。
3. `Concrete Modules`:
   - `apps/web/src/hooks/use_core/effects/message_dispatch_write.rs`
   - `apps/web/src/hooks/use_core/effects/message_repo_scope_accept.rs`
   - `apps/cli/src/server/handlers/sync/writer/mod.rs`
4. `Core Subsystems`:
   - `protocol`
   - `sync`

## Notes

- 这是 repo-scoped runtime flow，不是用户直接点击某个按钮才能发生的流程。
- 第一层仍保留为 operation 语义，只是这里的 operation 更接近“用户恢复当前 repo runtime 后触发的原子状态跃迁”。
- 当前示例只覆盖 handshake-ready 与 writer-ready，不扩展到后续 `SyncPush` / snapshot fallback。

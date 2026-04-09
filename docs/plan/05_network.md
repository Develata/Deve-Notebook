# 05_network.md - 网络工程蓝图

本章只定义连接、同步与会话恢复的工程实现，不描述用户提示文案。功能语义见 [../features/05_network.md](../features/05_network.md)，自动化验收见 [../acceptance-cases/06_network.md](../acceptance-cases/06_network.md)。

## 1. 目标

- 所有同步行为都必须是 `repo-scoped`。
- Web 端是 `WebLightPeer`，不是完整 ledger peer。
- 断连、未授权、降级同步必须在 runtime 中显式分流。

## 2. 权威实体

- `DashboardSession`
  - 用户登录态与访问权限。
- `PeerIdentity`
  - repo-scoped peer key / identity。
- `RepoScopedVector`
  - 当前 repo 的同步向量。
- `ScopeNonce`
  - 当前 repo/branch/doc 作用域代次。
- `ConnectionState`
  - WS transport 与 protocol lifecycle。

## 3. 分层

### 3.1 Authority

- repo vector、session verdict、scope binding 由 server/runtime 决定。
- 浏览器不拥有完整文档 authority。

### 3.2 Runtime Protocol

- 负责 `SyncHello / SyncRequest / SyncPush / Snapshot`。
- 所有进入同步路径的消息必须带可路由 `repo_id`。

### 3.3 Adapter

- WebSocket、HTTP fallback、Proxy/Main 转发属于 transport adapter。
- UI 只消费 runtime 暴露的连接状态，不直接操控 transport。

## 4. 状态机

### 4.1 连接状态

- `Disconnected`
- `Connecting`
- `HandshakePending`
- `Ready`
- `Reconnecting`
- `Unauthorized`
- `DegradedReadOnly`

### 4.2 转换规则

- `OpenPage -> Connecting`
- `WSOpen -> HandshakePending`
- `SyncHelloAccepted -> Ready`
- `SocketLost -> Reconnecting`
- `ReconnectExceededPolicy -> DegradedReadOnly`
- `401/403/AuthError -> Unauthorized`

## 5. Repo Scope 合同

- 连接绑定到单一当前 repo scope。
- 切换 repo 时必须重建 repo identity、vector、subscription 与 message routing。
- 一个 repo 的异常不得污染另一个 repo 的 scope state。

## 6. 存储分层

- `localStorage`
  - 仅存 UI 偏好与轻量 scope hint。
- `IndexedDB`
  - repo-scoped vector、cache metadata、identity metadata。
- `WebCrypto`
  - 私钥材料。
- `Server Ledger`
  - 文档与业务真相。

## 7. 恢复与修复

### 7.1 Reconnect

- 使用 backoff 重连。
- 重连成功后必须重新发送当前 repo 的 `SyncHello`。

### 7.2 Stale Scope

- 若持久化的 repo scope 无法恢复，客户端必须清除旧 scope 并请求健康 repo 列表。
- 不允许静默绑定到任意默认 repo。

### 7.3 Unauthorized

- 授权失败必须停止普通重连循环。
- UI 层只展示状态，不决定鉴权恢复策略。

## 8. 写入边界

- WebLightPeer 仅在 `Ready + write-ready scope` 下允许写入。
- 断连、未授权、spectator/remote scope 必须强制只读。
- 显示层不得仅凭“页面还开着”推断写入安全。

## 9. 禁止事项

- 禁止无 `repo_id` 的隐式同步路由。
- 禁止跨 repo 复用 peer identity、vector、scope state。
- 禁止把 `Unauthorized` 当成普通断网。
- 禁止显示层直接驱动 transport 实现细节。

## 10. 代码边界

- `crates/core/src/protocol/`
  - 协议消息与 repo-scoped message contract。
- `apps/cli/src/server/`
  - handshake、switcher、ws routing、auth/session integration。
- `apps/web/src/hooks/use_core/`
  - session runtime、scope runtime、message dispatch。

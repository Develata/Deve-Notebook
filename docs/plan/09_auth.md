# 09_auth.md - 认证工程蓝图

本章只定义 user session、peer identity、unauthorized/disconnected 分流与鉴权合同，不描述用户提示文案。功能语义见 [../features/09_auth.md](../features/09_auth.md)，自动化验收见 [../acceptance-cases/08_auth.md](../acceptance-cases/08_auth.md)。

## 1. 目标

- 明确区分 user session 与 peer identity 两层认证。
- 明确区分 `Unauthorized` 与 `Disconnected`。
- 任何写入都必须同时满足会话与作用域写入条件。

## 2. 权威实体

- `UserSession`
  - JWT / cookie / token version 驱动的访问授权。
- `PeerIdentity`
  - repo-scoped peer key / signing identity。
- `AuthVerdict`
  - 当前请求或连接的鉴权结论。

## 3. 两层认证

### 3.1 User Session

- 负责 dashboard / API / WS 访问权限。
- 生命周期包括：`Unauthenticated -> Authenticated -> Expired/Revoked`。

### 3.2 Peer Identity

- 负责 repo-scoped 同步来源与写入归属。
- 不能替代 user session 授权。

## 4. 状态机

- `Unauthenticated`
- `Authenticated`
- `PeerMissing`
- `Ready`
- `Unauthorized`
- `Disconnected`

### 转换规则

- `LoginOk -> Authenticated`
- `PeerReady -> Ready`
- `TokenExpired / 401 / 403 -> Unauthorized`
- `SocketLost -> Disconnected`

## 5. 协议合同

- HTTP 与 WebSocket 必须共享结构化错误码语义。
- `AUTH_*` 与 `401/403` 不得被包装成普通断网。
- repo-scoped write readiness 必须建立在 auth + scope runtime 基础上。

## 6. Session Probe

- `/api/auth/me` 周期探测应只在页面前台运行。
- 页面恢复到前台时应立即补探测一次。
- probe 只是 session 观测机制，不得直接充当业务真相。

## 7. 存储边界

- user session token 走 cookie/session 语义。
- peer identity 私钥不得暴露给显示层。
- 显示层不得自行推导“当前应该可写”，必须消费 runtime verdict。

## 8. 失败合同

- `Unauthorized`
  - 停止普通重连循环
  - 退出写态
- `Disconnected`
  - 可进入重连策略
  - 不等于认证失效
- `Session valid + Peer missing`
  - 只能进入受限/只读恢复链

## 9. 禁止事项

- 禁止 session token 充当 peer identity。
- 禁止把 unauthorized 伪装成 disconnected。
- 禁止显示层直接持有或操控鉴权真相。
- 禁止无结构化错误码的认证失败协议。

## 10. 代码边界

- `apps/cli/src/server/auth*`
  - session issuance / verification。
- `apps/cli/src/server/ws*` 与 handlers
  - ws auth entry、unauthorized verdict。
- `apps/web/src/hooks/use_core/`
  - auth/session runtime，供 UI 消费。

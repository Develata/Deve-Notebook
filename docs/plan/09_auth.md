# 09_auth.md - User Session 与入口鉴权工程蓝图

## Metadata

- `Layer`: `Runtime Protocols`
- `Status`: `Current MUST`
- `Counterpart Feature`: `docs/features/09_auth.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/08_auth.md`
- `Primary Code Areas`: `crates/core/src/security/auth/`, `apps/cli/src/server/auth/`, `apps/web/src/api/auth_probe.rs`, `apps/web/src/app_auth_monitor.rs`

## 1. Scope

本章定义：

- user session 的建立、续期、撤销、失效
- http/ws 入口鉴权
- unauthorized 与 disconnected 的严格分离
- JWT、cookie、安全头、CORS、CSRF、rate limit 的工程合同

peer identity、repo-scoped sync identity 与 pending write contract 不在本章主定义；它们分别由 `05_network` 与 `16_web_thin_client_ledger` 约束。

## 2. Authoritative Entities

### 2.1 Session Entities

- `AuthUser`
- `Claims`
- `SessionToken`
- `TokenVersion`
- `LoginAttemptWindow`

### 2.2 Security Context

- `HttpOnly Cookie`
- `Origin / Referer`
- `AllowedOrigins`
- `BruteForceWindow`
- `AuditLogEvent`

### 2.3 Layering Rule

- `User Session`
  - 回答“谁有权访问 API / Dashboard / WS”
- `Peer Identity`
  - 回答“同步数据来自哪个 peer”

二者是独立状态机，不允许互相替代。

## 3. State Machines

### 3.1 Session Lifecycle

```text
LoggedOut
  -> LoginRequested
  -> Authenticated
  -> SessionValid
  -> SessionExpired | LoggedOut
```

### 3.2 Browser Auth State

```text
Unknown
  -> Probing
  -> Authenticated
  -> Unauthorized
```

约束：

- `Unauthorized` 不得退化成普通断网态。
- `/api/auth/me` 只是 session probe，不得承担 peer identity 探测职责。

### 3.3 WS Entry Auth

```text
WsConnecting
  -> SessionVerified
  -> RepoHandshakePending
  -> AuthorizedWs
  -> Unauthorized
```

## 4. Commands / Endpoints / Outputs

### 4.1 HTTP Endpoints

- `POST /api/auth/login`
- `POST /api/auth/logout`
- `GET /api/auth/me`
- `GET /api/node/role`

### 4.2 Output Contracts

- `login`
  - 成功：签发 auth cookie，返回 success payload
  - 失败：结构化错误 + 失败审计日志
- `logout`
  - 清除 cookie
- `me`
  - 返回当前 session user

### 4.3 WS Gate

- ws 握手阶段必须验证 user session
- session invalid 时不得继续进入 repo/sync handshake

### 4.4 Endpoint Matrix

| Method | Path | Auth | Required Output |
| --- | --- | --- | --- |
| `POST` | `/api/auth/login` | No | set auth cookie or structured auth error |
| `POST` | `/api/auth/logout` | Yes | clear auth cookie |
| `GET` | `/api/auth/me` | Yes | current user/session payload |
| `GET` | `/api/node/role` | No | main/proxy diagnostic payload |

要求：

- 所有 auth endpoint MUST 返回稳定结构，不得以裸文本替代。
- ws upgrade 的 unauthorized 结果 MUST 与 http `401/403` 共享同一错误目录。

### 4.5 Bootstrapping Contract

- `No Init UI`
  - 首次启动不得依赖“初始化向导”创建认证基础设施。
  - auth 配置、secret、默认账号等必须通过环境变量或配置文件在启动前就绪。
- 若启动所需 auth material 缺失：
  - server MUST fail-closed
  - 不得自动进入匿名生产模式
  - 不得临时生成弱默认凭证继续启动

## 5. JWT and Cookie Contract

### 5.1 JWT

- algorithm：`HS256`
- payload 至少包含：
  - `sub`
  - `iat`
  - `exp`
  - `ver`

### 5.2 Cookie

- `HttpOnly`
- `SameSite=Strict`
- `Path=/`
- `Secure` 由 `HTTPS_ENABLED` 控制，但生产默认必须开启

### 5.3 Revocation

- `token_version` 变更后，旧 token 立即失效
- 密码修改必须触发该版本递增

### 5.4 JWT Payload and Delivery

- JWT payload 至少包含：
  - `sub`
  - `iat`
  - `exp`
  - `ver`
- 推荐 payload 形状：

```json
{
  "sub": "admin",
  "iat": 1700000000,
  "exp": 1700086400,
  "ver": 1
}
```

- cookie 交付格式至少满足：
  - `HttpOnly`
  - `SameSite=Strict`
  - `Path=/`
  - `Secure`（生产默认开启）

## 6. Access Control and Security Policies

### 6.1 Access Model

- 当前是 owner-only / single-user model
- localhost anonymous mode 只能是显式 dev 开关

### 6.2 CORS

- production：必须白名单
- development：可放宽到 localhost / 127.0.0.1
- 禁止默认 `allow_origin(Any)` 进入生产

### 6.3 CSRF

- 主防线：`SameSite=Strict`
- 辅助：可校验 `Origin` / `Referer`

### 6.4 Rate Limiting {#auth-rate-limiting}

- login endpoint：单独限流
- authenticated API：单独限流
- ws messages：单连接窗口限流

推荐基线：

- `POST /api/auth/login`
  - `5 req/min/IP`
- authenticated API
  - `120 req/min/IP`
- WebSocket messages
  - `200 msg/min/connection`

### 6.5 Security Headers {#security-headers}

> **Code Refs**: `apps/cli/src/server/auth/headers.rs`

所有 HTTP 响应必须包含：

- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self' ws: wss:; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; object-src 'none'; frame-ancestors 'none'; base-uri 'self'`

### 6.6 Audit

- 登录成功 / 失败必须记录结构化日志
- 至少包含：
  - IP
  - user
  - timestamp
  - user-agent（若可得）

### 6.7 Key and File Permissions

- 宿主机上的 `identity.key` / auth secret material 文件权限 MUST 为 owner-only。
- 浏览器端不得导出 peer private key 或 session material 到 localStorage / URL / logs。

### 6.8 Localhost / Dev Policy

- `AUTH_ALLOW_ANONYMOUS_LOCALHOST` 只能显式开启。
- 仅允许 `localhost` / `127.0.0.1` 的本地开发场景使用。
- 开启时 MUST 在日志中显著标记 dev-only auth bypass。

### 6.9 TLS Deployment Contract

- 推荐生产方案：
  - 由 reverse proxy（如 Caddy / Nginx）终止 TLS。
  - 内部 `deve serve` 保持 HTTP。
- 若直接由应用暴露 TLS：
  - 必须显式配置证书与私钥路径。
  - 所有 browser ws 入口必须升级为 `wss://`。
- 禁止：
  - 生产环境下通过明文 `ws://` 暴露跨公网 session traffic。
  - 让浏览器在 HTTPS 页面中降级连接明文 ws。

## 7. Session Probe Policy

- `/api/auth/me` 周期探测只应在前台活动页面进行
- 页面后台应暂停探测
- 页面回前台应立即补一次探测

## 8. Failure Modes

- invalid password
- missing cookie
- expired token
- revoked token
- anonymous localhost disabled
- brute-force lockout
- malformed origin / cors reject
- ws handshake unauthorized

错误目录建议：

- `AUTH_INVALID_CREDENTIALS`
- `AUTH_TOKEN_MISSING`
- `AUTH_TOKEN_EXPIRED`
- `AUTH_TOKEN_REVOKED`
- `AUTH_CSRF_REJECTED`
- `AUTH_ORIGIN_REJECTED`
- `AUTH_RATE_LIMITED`
- `AUTH_WS_REJECTED`

## 9. Recovery / Safety

### 9.1 Unauthorized Handling

- `401/403/AUTH_*` 必须进入 `Unauthorized`
- 客户端必须退出写态
- 需要重新登录，而不是继续普通重连

### 9.2 Disconnect Handling

- 纯网络断开只进入 `Disconnected`
- 可以自动重连
- 重连成功后仍需重新验证 session 与 repo handshake

### 9.3 Brute Force Recovery

- 达到阈值后封禁窗口生效
- 封禁窗口结束后允许重新尝试

### 9.4 Unauthorized vs Disconnected UI Contract

- `Unauthorized`
  - 立即退出写态
  - 停止普通重连
  - 显示登录 / 认证失效界面
- `Disconnected`
  - 保留 session 语义
  - 允许自动重连
  - 重连后重新执行 session probe + repo handshake

## 10. Forbidden Patterns

- 把 session token 当成 peer identity。
- 用 peer identity 绕过 API / ws 入口鉴权。
- 把 `Unauthorized` 伪装成断网。
- 在生产默认放开 CORS Any。
- 用裸字符串错误代替稳定 auth error contract。

## 11. Module Boundary

### 11.1 Core Security Layer

- `crates/core/src/security/auth/`

职责：

- jwt claims
- auth config
- password hashing

### 11.2 Server Auth Layer

- `apps/cli/src/server/auth/`
- `apps/cli/src/server/auth/handlers/`

职责：

- middleware
- login/logout/me handlers
- cookies
- brute force
- headers

### 11.3 Browser Auth Layer

- `apps/web/src/api/auth_probe.rs`
- `apps/web/src/app_auth_monitor.rs`
- `apps/web/src/components/login/`

职责：

- session probe
- login page
- unauthorized surface

## 12. Code Mapping

- core auth:
  - `crates/core/src/security/auth/jwt.rs`
  - `crates/core/src/security/auth/password.rs`
  - `crates/core/src/security/auth/config.rs`
- server auth:
  - `apps/cli/src/server/auth/middleware.rs`
  - `apps/cli/src/server/auth/cookie.rs`
  - `apps/cli/src/server/auth/brute_force.rs`
  - `apps/cli/src/server/auth/headers.rs`
  - `apps/cli/src/server/auth/handlers/login.rs`
  - `apps/cli/src/server/auth/handlers/session.rs`
- browser auth:
  - `apps/web/src/api/auth_probe.rs`
  - `apps/web/src/app_auth_monitor.rs`
  - `apps/web/src/components/login/`

## 13. Refactor Target

长期应显式形成：

- `session_runtime`
- `auth_gateway`
- `browser_auth_runtime`

当前实现已经有模块分层，但 unauthorized/disconnected/session-probe 语义仍然部分分散在前端 runtime 与 ws 路径中。未来重构应围绕这三层进一步收紧。

## 本章相关命令

- 无

## 本章相关配置 {#auth-config}

> **Code Refs**: `crates/core/src/security/auth/config.rs`

- `AUTH_SECRET`
- `AUTH_USER`
- `AUTH_PASS`
- `AUTH_ALLOW_ANONYMOUS_LOCALHOST`
- `AUTH_TOKEN_VERSION`
- `DEVE_ENV`
- `HTTPS_ENABLED`
- `ALLOWED_ORIGINS`

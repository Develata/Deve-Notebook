# 08_auth.md - User Session 与入口鉴权工程蓝图

## Metadata

- `Layer`: `Runtime Protocols`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-14`
- `Counterpart Feature`: `docs/features/09_auth.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/08_auth.md`
- `Primary Code Areas`: `crates/core/src/security/auth/`, `apps/cli/src/server/auth/`, `apps/web/src/api/auth_probe.rs`, `apps/web/src/app/auth_monitor.rs`

## 1. Scope

本章定义：

- user session 的建立、续期、撤销、失效
- http/ws 入口鉴权
- unauthorized 与 disconnected 的严格分离
- JWT、cookie、安全头、CORS、CSRF、rate limit 的工程合同

peer identity、repo-scoped sync identity 与 pending write contract 不在本章主定义；它们分别由 `07_network` 与 `09_web_thin_client_ledger` 约束。

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
  - 必须由 host identity 公钥推导；browser writer、session、plugin 或业务 actor 标签不能替代物理 PeerId

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
- `/api/auth/status` 是安静 session probe，不得承担 peer identity 探测职责。
- `/api/auth/me` 只返回当前 user/session payload，不用于首屏未登录探测。

### 3.3 WS Entry Auth

```text
WsConnecting
  -> SessionVerified
  -> RepoHandshakePending
  -> AuthorizedWs
  -> Unauthorized
```

## 4. Commands / Endpoints / Outputs

### 4.1 HTTP Endpoints {#auth-http-endpoints}

- `POST /api/auth/login`
- `POST /api/auth/logout`
- `GET /api/auth/status`
- `GET /api/auth/me`
- `GET /api/node/role`

### 4.2 Output Contracts

- `login`
  - 成功：签发 auth cookie，返回 success payload
  - 失败：结构化错误 + 失败审计日志
- `logout`
  - 清除 cookie
- `status`
  - 公开 session probe；无有效 session 时返回 `200 { authenticated: false }`，避免首屏未登录探测制造浏览器 401 噪音。
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
| `GET` | `/api/auth/status` | No | `{ authenticated: bool }` |
| `GET` | `/api/auth/me` | Yes | current user/session payload |
| `GET` | `/api/node/role` | No | main/proxy diagnostic payload |

要求：

- 所有 auth endpoint **MUST** 返回稳定结构，不得以裸文本替代。
- ws upgrade 的 unauthorized 结果 **MUST** 使用 `13_i18n.md#i18n-error-code-catalog` 的错误码目录。

### 4.5 Bootstrapping Contract

- `No Init UI`
  - 首次启动不得依赖“初始化向导”创建认证基础设施。
  - auth 配置、secret、默认账号等必须通过环境变量或配置文件在启动前就绪。
- 若启动所需 auth material 缺失：
  - server **MUST** fail-closed
  - 不得自动进入匿名生产模式
  - 不得临时生成弱默认凭证继续启动

## 5. JWT and Cookie Contract {#jwt-cookie-contract}

### 5.1 JWT

- algorithm：`HS256`
- payload 至少包含：
  - `sub`
  - `iat`
  - `exp`
  - `ver`
- 新签发 token **MUST** 额外包含每次登录唯一、不可预测的 `sid`。验证器 **MUST** 继续接受缺失
  `sid` 的旧 token；`sid` 只用于区分 browser auth session，不替代 `ver`、cookie 策略或 peer identity。

### 5.2 Cookie

- `HttpOnly`
- `SameSite=Strict`
- `Path=/`
- `Secure` 由 `HTTPS_ENABLED` 控制，但生产默认必须开启；只有显式 `0` / `false` / `no` / `off`
  才可关闭，非法值必须 fail-secure 为开启状态，不得静默降级为 insecure cookie。

### 5.2.1 Anonymous Localhost Dev Session Cookie

当有效 runtime environment 为 development（`deve serve --dev` 或 `DEVE_ENV=development`）、
`AUTH_ALLOW_ANONYMOUS_LOCALHOST=true` 且请求来自 loopback 地址时，
server MAY 签发匿名开发会话 cookie，用于把 HTTP 与 WebSocket runtime 绑定到同一个 browser
session。生产环境或未显式进入 development 时设置该开关必须 fail-closed，不能把 localhost
免密扩大成生产 credential。

约束：

- dev session cookie 只能在 anonymous localhost policy 下签发和接受，不得作为生产 auth credential。
- cookie value 必须包含 server 生成的不可预测 session nonce 与 server 可校验的 HMAC-SHA256 签名；`AuthSessionId`
  只能由已通过签名校验的 nonce 的不可逆 digest 派生，不得只由 username / token_version
  这类 dev-wide 固定值派生，也不得接受客户端自选的未签名 nonce。
- 同一请求同时携带有效 JWT cookie 与 anonymous localhost dev session cookie 时，JWT session
  必须优先成为 `AuthSessionId` 来源；dev session cookie 只作为无有效 JWT 时的本地开发 fallback。
- `/api/auth/status` 作为公开安静 probe，可在 anonymous localhost 下返回 `Set-Cookie`
  以建立 dev session；无有效 JWT 时仍必须返回 `200`，不得制造未登录 401 噪音。
- WebSocket Browser admission 在 anonymous localhost 下必须解析同一个 dev session cookie；
  若缺失，可在 upgrade response 中补发 cookie，但 FullPeer bearer admission 不得接受或签发该 cookie。
- protected HTTP middleware 在 anonymous localhost 下必须解析同一个 dev session cookie，并把
  派生出的 `AuthSessionId` 注入 request extension；缺失时可补发 cookie，但该新 session 不得匹配
  其他 browser session 已建立的 Source Control grant。
- anonymous localhost dev session cookie 必须复用本章 `HTTPS_ENABLED` cookie secure 策略；
  非显式关闭时应 fail-secure 为 `Secure`，不得形成与 JWT auth cookie 分叉的安全属性。
- remote proxy delegated Source Control API 不是 browser cookie authority；它必须使用单独的
  server-verifiable delegated capability。普通 JWT cookie、anonymous localhost dev cookie 或
  `REMOTE_PROXY_SCOPE_NONCE` 本身都不得授权 `/api/delegated/sc/*` 写入口。

### 5.3 Revocation

- `token_version` 变更后，旧 token 立即失效
- 密码修改必须触发该版本递增
- `AUTH_TOKEN_VERSION` 如显式设置，必须解析为有效 `u32` 版本号；非法值必须在启动配置加载阶段 fail-closed，
  不得静默回退到默认版本。

### 5.4 JWT Payload and Delivery

- JWT payload 至少包含：
  - `sub`
  - `iat`
  - `exp`
  - `ver`
- 新签发 token 还必须包含 per-login `sid`；旧 token 缺失 `sid` 时仍按 `sub` / `iat` /
  `exp` / `ver` 兼容验证。
- 推荐 payload 形状：

```json
{
  "sub": "admin",
  "iat": 1700000000,
  "exp": 1700086400,
  "ver": 1,
  "sid": "<per-login-random-session-id>"
}
```

- cookie 交付格式至少满足：
  - `HttpOnly`
  - `SameSite=Strict`
  - `Path=/`
  - `Secure`（生产默认开启）

### 5.5 Password Hashing {#password-hashing}

- `AUTH_PASS` **MUST** 存储为 Argon2 PHC string。
- 登录验证 **MUST** 将用户输入与配置中的 Argon2 hash 比较。
- 明文密码 **MUST NOT** 存储在 config、ledger、cookie、JWT 或 logs 中。

## 6. Access Control and Security Policies

### 6.1 Access Model

- 当前是 owner-only / single-user model
- localhost anonymous mode 只能是显式 dev 开关

### 6.2 CORS {#cors}

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

所有 HTTP 响应必须包含：

- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval' 'unsafe-inline'; connect-src 'self' ws: wss:; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; object-src 'none'; frame-ancestors 'none'; base-uri 'self'`
  - `script-src 'unsafe-inline'` 只是 Trunk/WASM bootstrapping 与本地 boot glue 的兼容例外；远程 script origin 仍然禁止。

### 6.6 Audit {#audit}

- 登录成功 / 失败必须记录结构化日志
- 至少包含：
  - IP
  - user
  - timestamp
  - user-agent（若可得）

### 6.7 Key and File Permissions {#key-and-file-permissions}

- 宿主机上的 `identity.key` / auth secret material 文件权限 **MUST** 为 owner-only。
- 所有本地 ledger fact writer 必须绑定由该 `identity.key` 推导出的物理 PeerId；`FactActor` 仅作诊断，不能参与签名、source proof 或 VersionVector。
- 浏览器端不得导出 peer private key 或 session material 到 localStorage / URL / logs。

### 6.8 Localhost / Dev Policy {#localhost-dev-policy}

- `AUTH_ALLOW_ANONYMOUS_LOCALHOST` 只能在显式 development mode（`deve serve --dev` 或
  `DEVE_ENV=development`）中开启；production / unset runtime environment 中设置为 true
  必须 fail-closed。
- 仅允许 `localhost` / `127.0.0.1` 的本地开发场景使用。
- 开启时 **MUST** 在日志中显著标记 dev-only auth bypass。
- anonymous localhost 虽绕过密码认证，但 browser session 隔离仍必须通过 dev session cookie 保持；
  Source Control write grant 不得退化为整个 localhost dev environment 共享的固定身份。

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

### 6.10 Native Shell IPC 与 Remote Session 隔离 {#native-shell-ipc-session-isolation}

- 远端 HTTPS 页面建立有效 JWT/browser session，只授权该远端 origin 的 HTTP/WS 入口；它不授予
  Desktop/Mobile 宿主 IPC、backend preference、local session、debug 或 service-control capability。
- native `LocalBackend` command surface 只能在可信 bundled origin 注册，并由 typed
  `NativeShellCapabilities.backend_preference_control` 与真实 native invoke 同时确认。单独存在
  `window.__TAURI_INTERNALS__` 不得被解释为 capability。
- native `RemoteBrowser` 不得注入 LocalBackend bootstrap capability，不得注册 application command
  handler 或 Web facade。远端页面的登录、编辑和 commit/history 必须仅经远端同源 HTTP/WS 完成。
- Desktop 从 `RemoteBrowser` 回到 `LocalBackend` 只能由 native-owned 菜单/托盘 coordinator 发起；
  Mobile 只能由 Android/iOS 平台原生恢复控件交给 native coordinator 发起。RemoteBrowser 的 auth
  cookie、endpoint、session、repo scope 与 `scope_nonce` 不得迁移或复用到新 LocalBackend runtime；
  切换前必须销毁远端 WebView，且远端页面始终不得用 DOM IPC 替代原生入口。
- 服务端 CSP 不得为了 native shell 模式切换而放宽 `connect-src` 到 `ipc.localhost`；正确实现必须让
  RemoteBrowser 页面根本不发出该请求。

## 7. Session Probe Policy {#session-probe-policy}

- `/api/auth/status` 周期探测只应在前台活动页面进行
- 页面后台应暂停探测
- 页面回前台应立即补一次探测
- 无有效 session 时，探测必须返回 `200 { authenticated: false }`，不得制造浏览器 401 噪音。

## 8. Failure Modes

- invalid password
- missing cookie
- expired token
- revoked token
- anonymous localhost disabled
- brute-force lockout
- malformed origin / cors reject
- ws handshake unauthorized
- RemoteBrowser 页面错误获得 native IPC/backend capability

错误码清单以 `13_i18n.md#i18n-error-code-catalog` 为唯一权威。

## 9. Recovery / Safety

### 9.1 Unauthorized Handling {#unauthorized-handling}

- `401/403/AUTH_*` 必须进入 `Unauthorized`
- 客户端必须退出写态
- 需要重新登录，而不是继续普通重连

### 9.2 Disconnect Handling

- 纯网络断开只进入 `Disconnected`
- 客户端必须撤销当前连接派生的 writer-ready / write grant 状态；重连前不得继续使用旧 `WriteReady`
- 可以自动重连
- 重连成功后仍需重新验证 session 与 repo handshake

### 9.3 Brute Force Recovery

- 达到阈值后封禁窗口生效
- 封禁窗口结束后允许重新尝试

### 9.4 Unauthorized vs Disconnected UI Contract {#unauthorized-disconnected-ui}

- `Unauthorized`
  - 立即退出写态
  - 停止普通重连
  - 显示登录 / 认证失效界面
- `Disconnected`
  - 保留 session 语义
  - 撤销当前连接派生的写态
  - 允许自动重连
  - 重连后重新执行 session probe + repo handshake

## 10. Forbidden Patterns

- 把 session token 当成 peer identity。
- 用 peer identity 绕过 API / ws 入口鉴权。
- 把 `Unauthorized` 伪装成断网。
- 在生产默认放开 CORS Any。
- 用裸字符串错误代替稳定 auth error contract。
- 把已认证远端 browser session 当成本机 native IPC capability。

## 11. Runtime Boundary

### 11.1 Core Security Layer

职责：

- jwt claims
- auth config
- password hashing

### 11.2 Server Auth Layer

职责：

- middleware
- login/logout/status/me handlers
- cookies
- brute force
- headers

### 11.3 Browser Auth Layer

职责：

- session probe
- login page
- unauthorized surface

## 12. Refactor Target

长期应显式形成：

- `session_runtime`
- `auth_gateway`
- `browser_auth_runtime`

后续重构应围绕这三层收紧 unauthorized / disconnected / session-probe 语义，避免前端 runtime 与 ws 路径形成隐式分叉。

## 本章相关命令

- 无

## 本章相关配置 {#auth-config}

- `AUTH_SECRET`
- `AUTH_USER`
- `AUTH_PASS`
- `AUTH_ALLOW_ANONYMOUS_LOCALHOST`
- `AUTH_TOKEN_VERSION`
- `DEVE_ENV`
- `HTTPS_ENABLED`
- `ALLOWED_ORIGINS`

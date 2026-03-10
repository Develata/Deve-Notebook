# 09_auth.md - 认证与登录篇 (Authentication)

## 认证策略 (Auth & Login)

*   **12-Factor Auth**：
    *   配置通过环境变量注入，**No Init UI** (无初始化界面，第一次启动即需环境变量就绪)。
*   **安全 (Security)**：
    *   **HTTPS**:
        *   **Public Network**: 必须强制 HTTPS。
        *   **Localhost/LAN**: **MAY** 允许 HTTP (开发/内网环境)，但需注意现代浏览器在非 HTTPS 环境下禁用 Crypto/Clipboard API 的限制。
    *   **Anti-CSRF**：必须实施抗 CSRF 策略。
    *   **Rate Limiting**：必须实施速率限制。
* **Protocol (机制)**:
    * **Auth Layering (分层认证)**: 明确区分 **User Session** 与 **Peer Identity**。
        * **User Session (JWT)**: 验证用户访问权限。采用 Stateless JWT，存储于 `HttpOnly Cookie`。Payload 包含 `sub: "admin"`, `exp`, `ver`。
        * **Peer Identity (Ed25519)**: 验证同步数据来源。每个 repo 独立的 keypair，存储于浏览器安全存储（WebCrypto/IndexedDB）。
    * **WebSocket Auth**: 握手阶段必须验证 JWT Token；同步阶段验证 Peer Identity 签名。
    * **Session**: 提供基于 Redis 或内存的会话管理机制（可选，视 JWT 策略而定）。
    * **2FA (Two-Factor Auth)**: **MAY** 支持 TOTP (Google Authenticator) 以增强安全性。

## 章节边界 (Scope Boundary)

本章只负责：

*   `user session` 的建立、续期、撤销与失效处理。
*   `WebSocket` / `HTTP` 入口的鉴权语义。
*   `Unauthorized` 与 `Disconnected` 的区分。

以下内容不在本章定义权威规则：

*   WebLightPeer 的 repo-scoped cache / storage 分层：见 `04_storage.md` 与 `05_network.md`。
*   Web 薄客户端的 pending overlay / ack / write readiness：见 `16_web_thin_client_ledger.md`。

## Auth-Specific Invariants (认证不变量)

1. `User session` 与 `peer identity` 是两层独立状态机，任何一层成功都不能替代另一层。
2. `session expired / token missing` **MUST** 进入 `Unauthorized`，而不是继续显示普通断网重连。
3. `session valid + peer identity missing` 只允许只读与重新注册，不允许写入。
4. `peer identity ready + session invalid` 仍然不得访问 API / WS 写路径。

## Auth Layering Flows (分层认证流程)

**Layer 1: User Session（用户会话）**

*   目的：回答“谁在访问 Dashboard / API”。
*   流程：`POST /api/auth/login` -> Server 签发 `session token` -> 浏览器以 `HttpOnly Cookie` 持有。
*   作用域：全局，覆盖该浏览器访问的所有 repos。
*   撤销：登出、Cookie 过期、密码修改、`token_version` 增加。
*   禁止：`session token` 不得充当 `peer identity`，也不得替代同步签名。

**Layer 2: Peer Identity（浏览器 Peer 身份）**

*   目的：回答“同步数据来自哪个 browser peer / repo”。
*   流程：浏览器在进入 repo 后生成或恢复 repo-scoped keypair，然后通过 `SyncHello` 完成 peer registration。
*   存储：私钥留在 `WebCrypto`，公钥、peer metadata 与 repo 状态在 IndexedDB。
*   作用域：严格 repo-scoped；切换 repo 必须切换对应 peer identity。
*   禁止：peer identity 不授予 UI/API 访问权限，不能绕过登录 Cookie。

## Unauthorized vs Disconnected (认证失效与断网分离)

*   **Disconnected**：网络、代理或服务暂时不可达。客户端 **MAY** 自动重连，并在恢复后重新握手当前 repo。
*   **Unauthorized**：JWT 缺失、过期、撤销或服务端明确拒绝当前会话。客户端 **MUST** 立即退出写态，跳转登录页或显示认证失效界面。
*   **Rule**：`401/403` 与 `AUTH_*` 错误码 **MUST NOT** 被重新包装成普通断网状态。
## 访问控制 (Access Control)

*   **Model**: **Single-User / Owner-Only**。
    *   **Algorithm**: `Argon2` (Pass hash) + `Ed25519` (Node Identity).
    *   **PeerID**: 基于公钥生成的唯一标识 (Hash of Public Key).
        *   **Implementation**: `SHA256(PublicKey)[0..12]` (Hex string).
        *   **Key Storage**: Native full peer 的 Private Key (Seed) stored in `ledger/.host/keys/identity.key`；每个本地 Repo 的共享 RepoKey stored in `vault/<repo_name>/.notegit/keys/repo.key`；浏览器 WebLightPeer **MUST NOT** 写入这些文件，而是使用 `WebCrypto + IndexedDB`。
        *   **Verification**: 握手消息 (Hello) 必须包含 Ed25519 签名，由接收方使用 PubKey 验证。
*   **Localhost Policy**:
    *   当通过 `localhost` 或 `127.0.0.1` 访问时，**MAY** 允许免密登录或自动填充默认凭据（Dev Mode），但必须有明确的配置开关 `AUTH_ALLOW_ANONYMOUS_LOCALHOST`。

## 安全策略 (Security Policies)

*   **CORS 策略**:
    *   **生产环境 (Production)**: Origin 限制为用户配置的域名白名单，**MUST NOT** 使用 `allow_origin(Any)`。
    *   **开发环境 (Development)**: **MAY** 放宽为 `http://localhost:{port}` 和 `http://127.0.0.1:{port}`，但 **MUST** 在日志中显著标记 `⚠ CORS: Dev-Mode (Relaxed)` 以提醒开发者。
    *   **切换条件**: 通过环境变量 `DEVE_ENV=production | development` 控制策略分支。
*   **Brute Force Protection**: 连续 5 次登录失败后 IP 封禁 15 分钟。
*   **Token Revocation**: 密码修改后所有已签发 JWT 立即失效 (通过 `token_version` 计数器机制)。
*   **Security Headers**: 所有 HTTP 响应 **MUST** 包含:
    *   `X-Content-Type-Options: nosniff`
    *   `X-Frame-Options: DENY`
    *   `Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'`
*   **Key File Permissions**: `identity.key` 文件权限 **MUST** 设为 `0600` (Owner-only)。
*   **Audit Log**: 登录成功/失败事件 **MUST** 记录到结构化日志 (Tracing)，包含 IP、User-Agent、Timestamp。

## JWT 规范 (JWT Specification)

*   **Algorithm**: `HS256` (using `AUTH_SECRET` as key)。
*   **Payload**:
    ```json
    {
      "sub": "admin",
      "iat": 1700000000,
      "exp": 1700086400,
      "ver": 1
    }
    ```
*   **Lifetime**: Access Token 有效期 `24h`；`ver` 字段用于 Token Revocation。
*   **Delivery**: `Set-Cookie: token=<jwt>; HttpOnly; Secure; SameSite=Strict; Path=/`。
*   **Refresh**: 客户端检测到 `401` 后重新登录（单用户场景无需 Refresh Token）。

## Anti-CSRF 策略

*   **Method**: `SameSite=Strict` Cookie 作为主要防御。
*   **Backup**: 对于非 GET 的状态变更请求，后端 **MAY** 额外校验 `Origin` 或 `Referer` Header。
*   **Note**: 因 `SameSite=Strict` 已阻止跨站请求，Double Submit Token 为可选增强。

## Rate Limiting 规范

*   **Login Endpoint** (`POST /api/auth/login`): 5 次/分钟/IP。
*   **API Endpoints**: 120 次/分钟/IP (Authenticated)。
*   **WebSocket**: 200 条消息/分钟/连接。
*   **Implementation**: 当前实现为自研内存限流器：HTTP 侧使用 per-IP 滑动窗口，WebSocket 侧使用每连接 60 秒窗口消息计数。

## TLS 配置

*   **推荐方案**: 反向代理 (Nginx/Caddy) 终止 TLS，内部 `deve serve` 仅 HTTP。
*   **直连方案 (可选)**: 支持 `--tls-cert` / `--tls-key` 参数直接启用 HTTPS。
*   **WebSocket**: 当 TLS 启用时，WS 自动升级为 `wss://`。

## API Endpoints

| Method | Path | Auth | Description |
|:---|:---|:---|:---|
| `POST` | `/api/auth/login` | No | 用户登录，返回 JWT Cookie |
| `POST` | `/api/auth/logout` | Yes | 清除 Cookie |
| `GET` | `/api/auth/me` | Yes | 返回当前用户信息 |
| `GET` | `/api/node/role` | No | 返回 Main/Proxy 角色信息 |

## 本章相关命令

* 无。

## 本章相关配置

*   `AUTH_SECRET`: JWT 签名密钥 (MUST >= 32 字节)。生产环境必须显式设置。
*   `AUTH_USER`: 用户名 (默认 "admin")。
*   `AUTH_PASS`: 密码的 Argon2 哈希 (PHC 格式)。生产环境必须显式设置。
*   `AUTH_ALLOW_ANONYMOUS_LOCALHOST`: 布尔。是否允许 localhost 免密访问 (默认 false)。
*   `AUTH_TOKEN_VERSION`: 整数。Token 版本号，修改密码后递增以撤销已签发 JWT。
*   `DEVE_ENV`: production | development。控制安全策略分支 (默认 production)。
*   `HTTPS_ENABLED`: 布尔。控制 Cookie Secure 属性 (默认 true)。
*   `ALLOWED_ORIGINS`: 逗号分隔的 CORS 白名单。生产环境必须显式设置。

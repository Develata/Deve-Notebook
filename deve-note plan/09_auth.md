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
*   **Protocol (机制)**:
    *   **JWT (JSON Web Token)**: 采用 Stateless JWT 进行身份凭证管理。
        *   **Payload**: 仅需包含 `sub: "admin"` 和 `exp`。
        *   **Storage**: 建议存储于 `HttpOnly Cookie` 以防御 XSS。
    *   **WebSocket Auth**: 必须并在握手阶段 (Handshake) 验证 Ticket/Token，拒绝未授权连接。
    *   **Session**: 提供基于 Redis 或内存的会话管理机制（可选，视 JWT 策略而定）。
    *   **2FA (Two-Factor Auth)**: **MAY** 支持 TOTP (Google Authenticator) 以增强安全性。

## 访问控制 (Access Control)

*   **Model**: **Single-User / Owner-Only**。
    *   **Algorithm**: `Argon2` (Pass hash) + `Ed25519` (Node Identity).
    *   **PeerID**: 基于公钥生成的唯一标识 (Hash of Public Key).
        *   **Implementation**: `SHA256(PublicKey)[0..12]` (Hex string).
        *   **Key Storage**: Private Key (Seed) stored in `vault/.deve/identity.key`.
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
*   **Implementation**: `tower_governor` 或 `tower::limit::RateLimitLayer`。

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

*   `AUTH_SECRET`: 用于签发 Session/JWT 的密钥。
*   `AUTH_USER`: 默认用户名 (env only).
*   `AUTH_PASS`: 默认密码 (env only).
*   `AUTH_ALLOW_ANONYMOUS_LOCALHOST`: 是否允许通过 `localhost` 或 `127.0.0.1` 访问时免密登录。

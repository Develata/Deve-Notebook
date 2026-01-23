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

## 本章相关命令

* 无。

## 本章相关配置

*   `AUTH_SECRET`: 用于签发 Session/JWT 的密钥。
*   `AUTH_USER`: 默认用户名 (env only).
*   `AUTH_PASS`: 默认密码 (env only).
*   `AUTH_ALLOW_ANONYMOUS_LOCALHOST`: 是否允许通过 `localhost` 或 `127.0.0.1` 访问时免密登录。

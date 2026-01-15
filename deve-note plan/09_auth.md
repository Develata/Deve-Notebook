# 09_auth.md - 认证与登录篇 (Authentication)

## 认证策略 (Auth & Login)

*   **12-Factor Auth**：
    *   配置通过环境变量注入，**No Init UI** (无初始化界面，第一次启动即需环境变量就绪)。
*   **安全 (Security)**：
    *   **HTTPS**：必须强制 HTTPS。
    *   **Anti-CSRF**：必须实施抗 CSRF 策略。
    *   **Rate Limiting**：必须实施速率限制。
*   **体验 (UX)**：
    *   **2FA**：可选支持。
    *   **Session**：提供会话管理机制。

## 本章相关命令

* 无。

## 本章相关配置

*   `AUTH_SECRET`: 用于签发 Session/JWT 的密钥。
*   `AUTH_USER`: 默认用户名 (env only).
*   `AUTH_PASS`: 默认密码 (env only).

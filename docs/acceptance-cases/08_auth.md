## 认证与安全

```markdown
- case_id: AUTH-001
  goal: 生产环境 Fail-Closed 启动 (C1 收口)。
  preconditions:
    - DEVE_ENV=production (或未设置)
    - 未设置 AUTH_SECRET 或 AUTH_PASS
  steps:
    - run: deve serve
  assertions:
    - exit_code_not_eq: 0
    - log_contains: "ERROR: Production mode requires AUTH_SECRET and AUTH_PASS"
    - log_not_contains: "generated random secret"

- case_id: AUTH-002
  goal: 显式开发模式启动 (C1 收口)。
  preconditions:
    - DEVE_ENV=development
    - 未设置 AUTH_SECRET
  steps:
    - run: deve serve
  assertions:
    - exit_code_eq: 0
    - log_contains: "WARNING: Development mode with default credentials"

- case_id: AUTH-003
  goal: Cookie Secure 策略 (H1 收口)。
  preconditions:
    - HTTPS_ENABLED=true
  steps:
    - run: curl -i -X POST http://localhost:3000/api/auth/login -H "Content-Type: application/json" -d "{\"username\":\"admin\",\"password\":\"secret\"}"
  assertions:
    - header_contains: "Set-Cookie: token=...; Secure; SameSite=Strict; HttpOnly"

- case_id: AUTH-004
  goal: CORS 环境驱动配置 (H1 收口)。
  preconditions:
    - DEVE_ENV=production
    - ALLOWED_ORIGINS="https://app.deve.com"
  steps:
    - run: curl -I -H "Origin: http://localhost:8080" http://localhost:3000/api/node/role
  assertions:
    - header_not_contains: "Access-Control-Allow-Origin: *"

- case_id: AUTH-005
  goal: 精确 Cookie 名称匹配 (M1 收口)。
  preconditions:
    - 请求携带 token_csrf 或 token_backup cookie
  steps:
    - run: curl -b "token_csrf=bad" http://localhost:3000/api/auth/me
  assertions:
    - http_status_eq: 401

- case_id: AUTH-006
  goal: localStorage Panic 防护 (M2 收口)。
  preconditions:
    - 浏览器禁用 localStorage 或存储空间满
  steps:
    - browser_disable_local_storage: true
    - browser_open: "/"
  assertions:
    - ui_assert: app_running true
    - log_contains: "WARNING: localStorage unavailable, falling back to memory"

- case_id: AUTH-007
  goal: CSRF 防护。
  preconditions:
    - 登录态存在
  steps:
    - run: curl -X POST http://127.0.0.1:3000/api/write -H "Origin: http://evil" -d "x=1"
  assertions:
    - http_status_in: [401, 403]

- case_id: AUTH-008
  goal: Rate Limiting 生效。
  preconditions:
    - 登录接口可访问
  steps:
    - run: powershell -Command "1..50 | % { curl -s -X POST http://127.0.0.1:3000/api/auth/login -H \"Content-Type: application/json\" -d '{\"username\":\"admin\",\"password\":\"bad\"}' }"
  assertions:
    - http_status_in: [429]

- case_id: AUTH-009
  goal: JWT payload 最小化。
  preconditions:
    - 登录成功获得 JWT
  steps:
    - run: deve auth decode-jwt --token <jwt>
  assertions:
    - jwt_claims_eq: ["sub", "iat", "exp", "ver"]

- case_id: AUTH-010
  goal: WebSocket 握手鉴权。
  preconditions:
    - 无有效 Token
  steps:
    - ws_connect: "ws://127.0.0.1:3000/ws"
  assertions:
    - ws_connection_denied true
    - http_status_eq: 401
    - json_field_eq: ["code", "AUTH_TOKEN_MISSING"]

- case_id: AUTH-011
  goal: session expired 与 disconnected 必须分离。
  preconditions:
    - 已登录并建立 WS 连接
  steps:
    - run: curl -s -X POST http://127.0.0.1:3000/api/auth/logout
    - browser_wait_ws_event: true
  assertions:
    - ui_assert: login_screen_visible true
    - ui_assert: overlay_text_not_eq "Reconnecting..."

- case_id: AUTH-012
  goal: 公开 session status 不产生未登录 401 噪音。
  preconditions:
    - 无有效 Token
  steps:
    - run: curl -i http://127.0.0.1:3000/api/auth/status
  assertions:
    - http_status_eq: 200
    - json_field_eq: ["authenticated", false]
```

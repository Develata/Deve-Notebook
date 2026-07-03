## 认证与安全

```markdown
- case_id: AUTH-001
  goal: 生产环境 Fail-Closed 启动 (C1 收口)。
  preconditions:
    - DEVE_ENV=production (或未设置)
    - 未设置 AUTH_SECRET 或 AUTH_PASS
  steps:
    - run: deve serve
    - run: scripts/check-auth-baseline.sh
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
    - run: scripts/check-auth-baseline.sh
  assertions:
    - exit_code_eq: 0
    - log_contains: "WARNING: development-only auth defaults active"

- case_id: AUTH-003
  goal: Cookie Secure 策略 (H1 收口)。
  preconditions:
    - HTTPS_ENABLED=true
  steps:
    - run: curl -i -X POST http://localhost:3000/api/auth/login -H "Content-Type: application/json" -d "{\"username\":\"admin\",\"password\":\"secret\"}"
    - run: scripts/check-auth-baseline.sh
    - run: cargo test -p deve_cli https_enabled_invalid_value_fails_secure -- --nocapture
    - run: cargo test -p deve_cli auth -- --nocapture
  assertions:
    - header_contains: "Set-Cookie: token="
    - header_contains: "Secure"
    - header_contains: "SameSite=Strict"
    - header_contains: "HttpOnly"
    - config_assert: invalid_HTTPS_ENABLED_fails_secure true

- case_id: AUTH-004
  goal: CORS 环境驱动配置 (H1 收口)。
  preconditions:
    - DEVE_ENV=production
    - ALLOWED_ORIGINS="https://app.deve.com"
  steps:
    - run: curl -i -H "Origin: http://localhost:8080" http://localhost:3000/api/node/role
    - run: scripts/check-auth-baseline.sh
  assertions:
    - header_not_contains: "Access-Control-Allow-Origin: *"
    - config_assert: ALLOWED_ORIGINS="*" fails closed with "Wildcard CORS origin is forbidden"

- case_id: AUTH-005
  goal: 精确 Cookie 名称匹配 (M1 收口)。
  preconditions:
    - 请求携带 token_csrf 或 token_backup cookie
  steps:
    - run: curl -b "token_csrf=bad" http://localhost:3000/api/auth/me
    - run: scripts/check-auth-baseline.sh
    - run: cargo test -p deve_cli auth -- --nocapture
  assertions:
    - http_status_eq: 401

- case_id: AUTH-006
  goal: localStorage Panic 防护 (M2 收口)。
  preconditions:
    - 浏览器禁用 localStorage 或存储空间满
  steps:
    - browser_disable_local_storage: true
    - browser_open: "/"
    - run: cargo test -p deve_web auth_probe -- --nocapture
  assertions:
    - ui_assert: app_running true
    - log_contains: "WARNING: localStorage unavailable, falling back to memory"

- case_id: AUTH-007
  goal: CSRF 防护。
  preconditions:
    - 跨站浏览器请求因 SameSite=Strict 不携带有效 auth cookie
  steps:
    - run: curl -X POST http://127.0.0.1:3000/api/sc/commit -H "Origin: http://evil" -H "Content-Type: application/json" -d "{\"message\":\"x\",\"targets\":[]}"
    - run: scripts/check-auth-baseline.sh
  assertions:
    - http_status_in: [401, 403]

- case_id: AUTH-008
  goal: Rate Limiting 生效。
  preconditions:
    - 登录接口可访问
  steps:
    - run: powershell -Command "1..50 | % { curl -s -X POST http://127.0.0.1:3000/api/auth/login -H \"Content-Type: application/json\" -d '{\"username\":\"admin\",\"password\":\"bad\"}' }"
    - run: scripts/check-auth-baseline.sh
    - run: cargo test -p deve_cli auth -- --nocapture
  assertions:
    - http_status_in: [429]

- case_id: AUTH-009
  goal: JWT payload 受控且每次登录 session 可区分。
  preconditions:
    - 登录成功获得 JWT
  steps:
    - run: scripts/check-auth-baseline.sh
    - run: cargo test -p deve_core issue_token_preserves_subject -- --nocapture
    - run: cargo test -p deve_core issue_token_mints_unique_session_id_per_login -- --nocapture
    - run: cargo test -p deve_core validate_token_accepts_legacy_payload_without_session_id -- --nocapture
    - run: cargo test -p deve_core invalid_auth_token_version_fails_closed -- --nocapture
    - run: cargo test -p deve_core auth -- --nocapture
  assertions:
    - jwt_claims_eq: ["sub", "iat", "exp", "ver", "sid"]
    - jwt_session_id_unique_per_login: true
    - legacy_jwt_without_sid_still_validates: true
    - config_assert: invalid_AUTH_TOKEN_VERSION_fails_closed true

- case_id: AUTH-010
  goal: WebSocket 握手鉴权。
  preconditions:
    - 无有效 Token
  steps:
    - ws_connect: "ws://127.0.0.1:3000/ws"
    - run: scripts/check-auth-baseline.sh
    - run: cargo test -p deve_cli ws::tests::unauthorized_ws_response_is_structured_json -- --nocapture
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
    - run: scripts/check-auth-baseline.sh
    - run: scripts/check-auth-unauthorized-state.sh
    - run: cargo run -p deve_baseline -- auth-unauthorized-state
    - run: cargo test -p deve_web writer_ready -- --nocapture
    - run: cargo test -p deve_web status_summary -- --nocapture
  assertions:
    - ui_assert: login_screen_visible true
    - ui_assert: overlay_text_not_eq "Reconnecting..."
    - writer_ready_cleared_on_unauthorized: true

- case_id: AUTH-012
  goal: 公开 session status 不产生未登录 401 噪音。
  preconditions:
    - 无有效 Token
  steps:
    - run: curl -i http://127.0.0.1:3000/api/auth/status
    - run: scripts/check-auth-baseline.sh
    - run: cargo test -p deve_cli auth -- --nocapture
  assertions:
    - http_status_eq: 200
    - json_field_eq: ["authenticated", false]

- case_id: AUTH-014
  goal: Anonymous localhost dev session cookie 绑定 HTTP 与 WS 会话。
  preconditions:
    - DEVE_ENV=development
    - AUTH_ALLOW_ANONYMOUS_LOCALHOST=true
    - 请求来自 loopback 地址
  steps:
    - run: cargo test -p deve_cli anonymous_localhost_status_sets_dev_session_cookie -- --nocapture
    - run: cargo test -p deve_cli anonymous_localhost_ws_uses_dev_session_cookie -- --nocapture
    - run: cargo test -p deve_cli anonymous_localhost_auth_prefers_valid_jwt_over_dev_session_cookie -- --nocapture
    - run: cargo test -p deve_cli rejects_forged_dev_session_cookie -- --nocapture
    - run: cargo test -p deve_cli dev_session_cookie_is_bound_to_signing_secret -- --nocapture
    - run: cargo test -p deve_cli dev_session_cookie_secure_follows_policy -- --nocapture
    - run: scripts/check-auth-baseline.sh
  assertions:
    - http_status_eq: 200
    - header_contains: "Set-Cookie: deve_dev_session="
    - header_contains_unless_explicit_http: "Secure"
    - api_assert: anonymous_localhost_dev_session_cookie_is_per_browser_session true
    - api_assert: valid_jwt_session_takes_precedence_over_dev_session_cookie true
    - api_assert: forged_dev_session_cookie_is_replaced true
    - api_assert: dev_session_cookie_is_bound_to_server_secret true
    - api_assert: dev_session_cookie_secure_policy_matches_auth_cookie true

- case_id: AUTH-015
  goal: Anonymous localhost 不能在 production / unset DEVE_ENV 中启用。
  preconditions:
    - DEVE_ENV=production 或未设置
    - AUTH_SECRET 与 AUTH_PASS 均有效
    - AUTH_ALLOW_ANONYMOUS_LOCALHOST=true
  steps:
    - run: scripts/check-auth-baseline.sh
    - run: cargo test -p deve_core anonymous_localhost_requires_development_env -- --nocapture
  assertions:
    - config_assert: production_AUTH_ALLOW_ANONYMOUS_LOCALHOST_fails_closed true

- case_id: AUTH-013
  goal: Host identity key owner-only 权限。
  preconditions:
    - Unix-like host
    - identity.key 存在或由启动流程生成
  steps:
    - run: cargo test -p deve_cli identity_key_permissions -- --nocapture
    - run: scripts/check-auth-baseline.sh
  assertions:
    - cli_assert: identity_key_permission_corrected_to_0600 true
    - cli_assert: identity_key_non_file_fails_closed true
```

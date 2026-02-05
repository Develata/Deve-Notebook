## 认证与安全

```markdown
- case_id: AUTH-001
  goal: 12-Factor Auth 环境变量注入。
  preconditions:
    - 未设置 AUTH_SECRET
  steps:
    - run: deve serve
  assertions:
    - log_contains_any: ["AUTH_SECRET missing", "generated random secret"]

- case_id: AUTH-002
  goal: 公网强制 HTTPS。
  preconditions:
    - 服务绑定公网地址
  steps:
    - run: curl -I http://public.example.com
  assertions:
    - http_status_in: [301, 302, 307, 308]
    - header_contains: "Location: https://"

- case_id: AUTH-003
  goal: Localhost 可选免密。
  preconditions:
    - AUTH_ALLOW_ANONYMOUS_LOCALHOST=true
  steps:
    - run: curl -I http://127.0.0.1:3000
  assertions:
    - http_status_eq: 200

- case_id: AUTH-004
  goal: CSRF 防护。
  preconditions:
    - 登录态存在
  steps:
    - run: curl -X POST http://127.0.0.1:3000/api/write -H "Origin: http://evil" -d "x=1"
  assertions:
    - http_status_in: [401, 403]

- case_id: AUTH-005
  goal: Rate Limiting 生效。
  preconditions:
    - 登录接口可访问
  steps:
    - run: powershell -Command "1..50 | % { curl -s http://127.0.0.1:3000/api/login }"
  assertions:
    - http_status_in: [429]

- case_id: AUTH-006
  goal: JWT payload 最小化。
  preconditions:
    - 登录成功获得 JWT
  steps:
    - run: deve auth decode-jwt --token <jwt>
  assertions:
    - jwt_claims_eq: ["sub", "exp"]

- case_id: AUTH-007
  goal: WebSocket 握手鉴权。
  preconditions:
    - 无有效 Token
  steps:
    - ws_connect: "ws://127.0.0.1:3000/ws"
  assertions:
    - ws_connection_denied true
```

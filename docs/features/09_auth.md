# 09_auth.md - 登录与会话体验篇

本章描述登录、会话失效、未授权与断连的用户可见行为。

## 功能目标

- 用户应能区分登录成功、登录失败、会话失效与普通断网。
- 未授权状态不能伪装成“只是暂时重连中”。

## Operation 示例

- 原子操作建模示例见 `docs/features/operations/auth_login.md`。
- 该示例将“登录”拆为用户名输入、密码输入、提交、结果接收四个 user operations，用于支撑架构蓝图的第一层。
- 会话失效 / 未授权闭环示例见 `docs/features/operations/auth_session_unauthorized.md`。
- 该示例将“session expired / unauthorized”拆为恢复工作区、发起受保护请求、收到未授权、进入重新认证界面四个 user operations。

## 功能项

### 1. 登录与登出

- 登录成功后进入受保护工作区。
- 登录失败时显示明确失败结果。
- 登出后不应继续保留受保护写态。
- 每次登录建立独立 browser auth session；即使同一用户在同一秒内重复登录，也不得复用同一个可写 session 身份。

### 2. 会话失效

- 会话过期或被撤销时，用户应看到明确的认证失效结果。
- 写态必须被撤销。

### 3. Unauthorized vs Disconnected

- 未授权与断网必须可见区分。
- 断网可以等待重连；未授权则需要重新认证。
- 两者都不得继续沿用旧连接产生的写入就绪状态。

### 4. Anonymous Localhost Dev Session

- 在 `DEVE_ENV=development` 中显式开启 anonymous localhost 后，浏览器仍应获得 per-session dev cookie。
- 该 cookie 只用于本地开发会话隔离，不等同于生产 JWT；cookie value 必须由 server 以 HMAC-SHA256 签名。
- 生产环境或未显式进入 development 时设置 anonymous localhost 必须 fail-closed，不能形成生产免密登录。
- 该 cookie 与 JWT auth cookie 使用同一个 `HTTPS_ENABLED` Secure 策略；除非显式关闭，否则应带 `Secure`。
- 已登录浏览器同时携带有效 JWT 与 dev cookie 时，JWT session 必须优先；dev cookie 不能把已登录 HTTP
  请求降级成另一个 anonymous dev session。
- Source Control HTTP write grant 必须绑定到同一个 dev browser session；另一个 localhost browser/profile/script
  缺少相同且签名有效的 dev session cookie 时，不应复用该 grant。

### 5. Native RemoteBrowser 会话隔离

- Desktop/Mobile RemoteBrowser 中的正常远端登录只建立远端 browser session，不开放宿主 native IPC。
- 普通 Docker 浏览器和 RemoteBrowser 即使运行环境存在 `__TAURI_INTERNALS__`，没有 typed bundled-local
  capability 时也不得注册 backend facade。
- 远端页面不能用 IPC 切回 LocalBackend。Desktop 只能通过原生菜单/托盘恢复；Mobile 只能通过
  Android/iOS 平台原生控件交给 native coordinator 恢复，不以 Web fallback 伪装。
- 切回 LocalBackend 后必须建立新 endpoint/session/scope；远端 auth cookie 与旧 authority 不得复用。

## 非目标

- 当前阶段不把 peer identity 暴露为用户层可见登录概念。
- 当前阶段不允许未授权状态继续假装可写。
- anonymous localhost dev cookie 不是远程访问凭据，也不允许扩大为生产免密登录。
- 远端 browser auth session 不是 native IPC 或本机 backend preference capability。

## Chrome MCP 验收实例

### AUTH-FEAT-01: 登录成功与失败

前置条件：

- 应用处于登录入口或受保护页面。

步骤：

1. 使用正确凭据登录。
2. 登出。
3. 使用错误凭据再次登录。

期望结果：

- 正确凭据可进入应用。
- 错误凭据明确失败。

### AUTH-FEAT-02: 会话失效

前置条件：

- 已处于登录状态。

步骤：

1. 使当前会话过期或失效。
2. 在页面中继续触发受保护请求。

期望结果：

- 页面进入未授权状态。
- 不会继续显示普通重连。

### AUTH-FEAT-03: 未授权与断网分离

前置条件：

- 已处于受保护页面。

步骤：

1. 模拟一次普通断网。
2. 恢复网络。
3. 再模拟一次明确的 401/403。

期望结果：

- 断网表现为重连/离线提示。
- 401/403 表现为需要重新认证，而不是继续重连。

### AUTH-FEAT-04: RemoteBrowser 不获得宿主 IPC

前置条件：

- 普通 Docker 浏览器与 preference-driven Desktop RemoteBrowser 均连接同一 HTTPS 测试 origin。

步骤：

1. 分别完成登录、编辑和 commit/history。
2. 重新加载页面并采集 network/console。
3. 从 Desktop 原生菜单切回 LocalBackend。

期望结果：

- 两种远端页面都没有 native backend facade 或 `ipc.localhost` 请求，也没有相关 CSP 错误。
- Desktop 新进程建立新的 LocalBackend session/scope，旧远端 auth/authority 不复用。

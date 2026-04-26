# auth_session_unauthorized.md - 会话失效 / 未授权操作流示例

## Metadata

- `Flow ID`: `flow.auth.session-unauthorized`
- `Domain`: `auth`
- `Related Feature Chapters`: `docs/features/09_auth.md`
- `Related Acceptance Cases`: `AUTH-002`, `AUTH-003`, `AUTH-011`, `AUTH-012`

## Operations

### `op.auth.session.resume-workspace`

- `Name`: `Resume Protected Workspace`
- `Surface`: `workspace-shell`
- `Trigger`: 用户重新聚焦标签页、恢复前台页面、继续停留在受保护工作区
- `Preconditions`: 当前 UI 处于 `Authenticated`
- `Immediate Result`: 前端允许 session probe 继续运行
- `Application Entry`: `apps/web/src/app.rs`, `apps/web/src/app_auth_monitor.rs`

### `op.auth.session.issue-protected-request`

- `Name`: `Issue Protected Request`
- `Surface`: `workspace-runtime`
- `Trigger`: 用户继续执行受保护操作，或运行中的 WS / HTTP 路径继续发送请求
- `Preconditions`: 当前 UI 仍认为 session 可用
- `Immediate Result`: 请求进入 `/api/auth/status` probe 或 repo-scoped protocol path
- `Application Entry`: `apps/web/src/api/auth_probe.rs`, `apps/web/src/api/connection.rs`, `apps/web/src/hooks/use_core/effects/message_protocol.rs`

### `op.auth.session.receive-unauthorized`

- `Name`: `Receive Unauthorized Result`
- `Surface`: `workspace-runtime`
- `Trigger`: `401`、`403`、`AuthTokenExpired`、`AuthTokenMissing`
- `Preconditions`: `op.auth.session.issue-protected-request` 已执行
- `Immediate Result`: 连接状态切换为 `Unauthorized`，或 `AuthState` 切换为 `Unauthenticated`
- `Application Entry`: `apps/web/src/api/auth_probe.rs`, `apps/web/src/api/service.rs`, `apps/web/src/components/main_layout_setup.rs`

### `op.auth.session.enter-reauth-surface`

- `Name`: `Enter Reauthentication Surface`
- `Surface`: `login-surface`
- `Trigger`: unauthorized 状态被前端 runtime 观察到
- `Preconditions`: `op.auth.session.receive-unauthorized` 已发生
- `Immediate Result`: 主布局退出，回到登录页或明确的认证失效界面
- `Application Entry`: `apps/web/src/app.rs`, `apps/web/src/components/main_layout.rs`

## Response Flows

### `op.auth.session.resume-workspace`

1. `User Operation`: 用户重新回到受保护工作区。
2. `Application Response`: `App` 根据 `page_active` 与 `AuthState` 决定是否允许 session probe。
3. `Concrete Modules`:
   - `apps/web/src/app.rs`
   - `apps/web/src/app_auth_monitor.rs`
4. `Core Subsystems`: 无。此步只决定是否继续探测，不直接进入核心鉴权。

### `op.auth.session.issue-protected-request`

1. `User Operation`: 用户继续停留在工作区并触发受保护请求。
2. `Application Response`: 前端发起 `/api/auth/status`，或继续沿 WS / protocol 路径处理 server error。
3. `Concrete Modules`:
   - `apps/web/src/api/auth_probe.rs`
   - `apps/web/src/api/connection.rs`
   - `apps/web/src/hooks/use_core/effects/message_protocol.rs`
4. `Core Subsystems`:
   - `security`
   - `protocol`

### `op.auth.session.receive-unauthorized`

1. `User Operation`: 用户观察到当前受保护操作返回未授权。
2. `Application Response`: `401` 映射为 `AuthProbe::Invalid`；WS auth error 调用 `mark_unauthorized`，明确区分 unauthorized 与 disconnected。
3. `Concrete Modules`:
   - `apps/web/src/api/auth_probe.rs`
   - `apps/web/src/api/service.rs`
   - `apps/web/src/hooks/use_core/effects/message_protocol.rs`
   - `crates/core/src/protocol/auth.rs`
4. `Core Subsystems`:
   - `security`
   - `protocol`

### `op.auth.session.enter-reauth-surface`

1. `User Operation`: 用户被带回重新认证入口。
2. `Application Response`: `MainLayout` 观察 `ConnectionStatus::Unauthorized`，触发 `on_session_expired`；根 `App` 切换到 `Unauthenticated`，退出受保护布局。
3. `Concrete Modules`:
   - `apps/web/src/components/main_layout.rs`
   - `apps/web/src/components/main_layout_setup.rs`
   - `apps/web/src/app.rs`
4. `Core Subsystems`: 无。此步是 unauthorized surface 的 UI 收口。

## Notes

- 该 flow 的重点不是“自动重连”，而是把 `unauthorized` 与 `disconnected` 明确分离。
- 触发来源可以是定时 session probe，也可以是任意受保护 WS / HTTP 请求。
- 该 flow 与 `auth_login.md` 一起组成 auth 状态机闭环。

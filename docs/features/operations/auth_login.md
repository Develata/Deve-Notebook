# auth_login.md - 登录操作流示例

## Metadata

- `Flow ID`: `flow.auth.login`
- `Domain`: `auth`
- `Related Feature Chapters`: `docs/features/09_auth.md`
- `Related Acceptance Cases`: `AUTH-001`, `AUTH-006`, `AUTH-011`

## Operations

### `op.auth.login.type-username`

- `Name`: `Type Username`
- `Surface`: `web-form`
- `Trigger`: `input#login-username`
- `Preconditions`: 登录页已显示
- `Immediate Result`: 前端用户名状态更新
- `Application Entry`: `apps/web/src/components/login/page.rs`

### `op.auth.login.type-password`

- `Name`: `Type Password`
- `Surface`: `web-form`
- `Trigger`: `input#login-password`
- `Preconditions`: 登录页已显示
- `Immediate Result`: 前端密码状态更新
- `Application Entry`: `apps/web/src/components/login/page.rs`

### `op.auth.login.submit`

- `Name`: `Submit Login Form`
- `Surface`: `web-form`
- `Trigger`: 点击 Login 按钮或在表单中按 Enter
- `Preconditions`: 用户名与密码字段已存在输入值
- `Immediate Result`: 进入 `Authenticating` 状态并发起 `POST /api/auth/login`
- `Application Entry`: `apps/web/src/components/login/page.rs`, `apps/web/src/api/auth_login.rs`, `apps/cli/src/server/auth/handlers/login.rs`

### `op.auth.login.receive-result`

- `Name`: `Receive Login Result`
- `Surface`: `web-form`
- `Trigger`: 登录请求返回
- `Preconditions`: `op.auth.login.submit` 已执行
- `Immediate Result`: 前端切换到 `Authenticated` 或 `Failed`
- `Application Entry`: `apps/web/src/api/auth_login.rs`, `apps/web/src/components/login/page.rs`

## Response Flows

### `op.auth.login.type-username`

1. `User Operation`: 用户在用户名输入框键入字符。
2. `Application Response`: `on:input` 调用 `set_username` 更新本地 signal。
3. `Concrete Modules`: `components::login::page`
4. `Core Subsystems`: 无。此步只更新前端局部状态，不进入核心鉴权子系统。

### `op.auth.login.type-password`

1. `User Operation`: 用户在密码输入框键入字符。
2. `Application Response`: `on:input` 调用 `set_password` 更新本地 signal。
3. `Concrete Modules`: `components::login::page`
4. `Core Subsystems`: 无。此步只更新前端局部状态，不进入核心鉴权子系统。

### `op.auth.login.submit`

1. `User Operation`: 用户点击 Login 或按 Enter 提交表单。
2. `Application Response`: 表单 `on:submit` 阻止默认行为，先检查空字段，再把 `AuthState` 置为 `Authenticating`，随后调用 `attempt_login`。
3. `Concrete Modules`:
   - `apps/web/src/components/login/page.rs`
   - `apps/web/src/api/auth_login.rs`
   - `apps/cli/src/server/auth/handlers/login.rs`
   - `apps/cli/src/server/auth/handlers/session.rs`
   - `crates/core/src/security/auth/config.rs`
   - `crates/core/src/security/auth/password.rs`
   - `crates/core/src/security/auth/jwt.rs`
4. `Core Subsystems`:
   - `security`
   - `protocol`

### `op.auth.login.receive-result`

1. `User Operation`: 用户等待请求返回并观察结果。
2. `Application Response`: 成功时进入 `Authenticated`；失败时映射为结构化错误文案；服务端成功时写入 auth cookie，失败时返回结构化 auth error。
3. `Concrete Modules`:
   - `apps/web/src/components/login/page.rs`
   - `apps/web/src/api/auth_login.rs`
   - `apps/cli/src/server/auth/handlers/login.rs`
   - `apps/cli/src/server/auth/handlers/session.rs`
4. `Core Subsystems`:
   - `security`
   - `protocol`

## Submit 的内部模块流

`op.auth.login.submit` 的细化流向：

1. `login page` 检查空字段并发起请求。
2. `login api` 构造 `LoginRequest`，发送到 `/api/auth/login`。
3. `login handler` 执行暴力破解限流检查。
4. `login handler` 比较用户名。
5. `security::auth::password` 校验 Argon2 密码哈希。
6. `security::auth::jwt` 签发 token。
7. `session handler` 写入 `Set-Cookie`。
8. 前端根据 `LoginResponse` 切换 `AuthState`。

## Notes

- `login` 不是一个单层节点，而是一组原子操作组成的 flow。
- 第一层应优先展示 `type-username`、`type-password`、`submit`、`receive-result`，而不是只写 `login`。
- 若以后加入 CLI 登录或 trusted agent 登录，应复用 `submit-login` 这一意图名，但补充不同 `surface` 与 `application entry`。

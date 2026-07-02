# sc_commit.md - Source Control Commit 操作流示例

## Metadata

- `Flow ID`: `flow.sc.commit`
- `Domain`: `source-control`
- `Related Feature Chapters`: `docs/features/07_diff_logic.md`, `docs/features/10_ai_agent.md`
- `Related Acceptance Cases`: `DIFF-FEAT-01`, `DIFF-FEAT-03`, `DIFF-009`, `AI-007`

## Operations

### `op.sc.commit.focus-input`

- `Name`: `Focus Commit Input`
- `Surface`: `source-control-panel`
- `Trigger`: 用户打开 Source Control 并点击 commit message 输入框
- `Preconditions`: 当前 repo 已加载，Source Control 面板可见
- `Immediate Result`: commit message 输入区进入编辑态
- `Application Entry`: `apps/web/src/components/sidebar/source_control/commit.rs`, `apps/web/src/components/sidebar/source_control/commit_controller.rs`

### `op.sc.commit.type-message`

- `Name`: `Type Commit Message`
- `Surface`: `source-control-panel`
- `Trigger`: 用户在 commit message 输入框中键入内容
- `Preconditions`: `op.sc.commit.focus-input` 已发生
- `Immediate Result`: 本地 commit message state 更新
- `Application Entry`: `apps/web/src/components/sidebar/source_control/commit_controller.rs`

### `op.sc.commit.submit`

- `Name`: `Submit Commit`
- `Surface`: `source-control-panel`
- `Trigger`: 点击 commit 按钮，或执行等价命令
- `Preconditions`: staged changes 或 confirmed ledger changes 非空，message 非空，write gate 未阻塞
- `Immediate Result`: 前端发送 `ClientMessage::Commit { scope_nonce }`
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks_sc/write/commit.rs`, `apps/cli/src/server/ws/route/source_control.rs`, `apps/cli/src/server/handlers/source_control/commits.rs`

### `op.sc.commit.generate-message`

- `Name`: `Generate Commit Message`
- `Surface`: `source-control-panel`
- `Trigger`: 点击 commit message 区域的 generate 按钮
- `Preconditions`: staged changes 或 confirmed ledger changes 非空，write gate 未阻塞
- `Immediate Result`: 按 server AI backend capability gate 选择可用后端；不可用时显示原因并停止 generating/loading，不发起 plugin call
- `Application Entry`: `apps/web/src/components/sidebar/source_control/commit_ai.rs`, `apps/cli/src/server/handlers/plugin.rs`

### `op.sc.commit.receive-result`

- `Name`: `Receive Commit Result`
- `Surface`: `source-control-panel`
- `Trigger`: 服务端返回 commit ack，或返回 source control error / write gate failure
- `Preconditions`: `op.sc.commit.submit` 已执行
- `Immediate Result`: staged 区被清空并刷新历史，或显示明确错误
- `Application Entry`: `apps/web/src/hooks/use_core/effects_sc.rs`, `apps/web/src/hooks/use_core/effects/message_protocol/mod.rs`, `apps/web/src/components/sidebar/source_control/error_notice.rs`

## Response Flows

### `op.sc.commit.focus-input`

1. `User Operation`: 用户进入 Source Control commit 输入区。
2. `Application Response`: commit controller 准备 message state 与按钮可用性。
3. `Concrete Modules`:
   - `apps/web/src/components/sidebar/source_control/commit.rs`
   - `apps/web/src/components/sidebar/source_control/commit_controller.rs`
4. `Core Subsystems`: 无。此步只建立 UI 编辑态。

### `op.sc.commit.type-message`

1. `User Operation`: 用户输入提交说明。
2. `Application Response`: message signal 更新，`can_commit_now` 重新计算。
3. `Concrete Modules`:
   - `apps/web/src/components/sidebar/source_control/commit_controller.rs`
4. `Core Subsystems`: 无。此步只更新前端局部状态。

### `op.sc.commit.submit`

1. `User Operation`: 用户点击 commit。
2. `Application Response`: write gate 先检查 `session expired / offline / readonly / scope switching / handshaking repo`；通过后发送 repo-scoped `Commit` 消息。
3. `Concrete Modules`:
   - `apps/web/src/hooks/use_core/write_gate/logic.rs`
   - `apps/web/src/hooks/use_core/callbacks_sc/write/commit.rs`
   - `apps/cli/src/server/ws/route/source_control.rs`
   - `apps/cli/src/server/handlers/source_control/commits.rs`
   - `apps/cli/src/server/handlers/source_control/service/write.rs`
   - `crates/core/src/ledger/manager/commit_runtime.rs`
4. `Core Subsystems`:
   - `source_control`
   - `ledger`
   - `protocol`

### `op.sc.commit.generate-message`

1. `User Operation`: 用户请求生成提交说明。
2. `Application Response`: 先执行 AI backend capability gate；可用时复用 chat plugin stream，失败时只显示原因，不写入 commit message。
3. `Concrete Modules`:
   - `apps/web/src/components/sidebar/source_control/commit_ai.rs`
   - `apps/web/src/api/ai_backend.rs`
   - `apps/cli/src/server/handlers/plugin.rs`
4. `Core Subsystems`:
   - `source_control`
   - `ai`

### `op.sc.commit.receive-result`

1. `User Operation`: 用户观察 commit 返回结果。
2. `Application Response`: 成功时刷新 `changes / history` 并清空 staged 与 confirmed ledger changes；失败时进入 source control notice 或 protocol error surface。
3. `Concrete Modules`:
   - `apps/web/src/hooks/use_core/effects_sc.rs`
   - `apps/web/src/hooks/use_core/effects/message_protocol/mod.rs`
   - `apps/web/src/components/sidebar/source_control/error_notice.rs`
   - `apps/cli/src/server/handlers/source_control/errors/`
4. `Core Subsystems`:
   - `source_control`
   - `protocol`

## Notes

- 这条 flow 只描述 commit，不重复 stage / unstage。
- 首版 confirmed ledger changes 采用整锚 commit，不提供逐文件 include/exclude。
- commit 的关键 gate 不是按钮本身，而是 `write gate + scope_nonce + repo-scoped authority`。
- AI 生成提交说明只是辅助输入，不得绕过 AI backend capability gate，也不得在失败时写入 commit message。
- 成功 commit 的最终成立条件仍是 ledger append 与 commit anchor，而不是 UI 列表刷新。

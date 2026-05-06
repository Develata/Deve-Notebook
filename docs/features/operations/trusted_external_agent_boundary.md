# trusted_external_agent_boundary.md - Trusted External Agent 边界流示例

## Metadata

- `Flow ID`: `flow.ai.trusted-external-agent-boundary`
- `Domain`: `ai`
- `Related Feature Chapters`: `docs/features/10_ai_agent.md`, `docs/features/17_plugins.md`
- `Related Acceptance Cases`: `AI-005`, `AI-006`, `PLUG-001`

## Operations

### `op.ai.trusted-agent.open-settings`

- `Name`: `Open Trusted Agent Settings Surface`
- `Surface`: `settings-or-extensions`
- `Trigger`: 用户打开 Settings 或 Extensions，查看 AI backend / Trusted External Agent 状态
- `Preconditions`: 主界面已加载
- `Immediate Result`: 展示 backend 选项与 trusted/default-off 提示
- `Application Entry`: `apps/web/src/components/settings_sections.rs`, `apps/web/src/components/sidebar/extensions.rs`

### `op.ai.trusted-agent.select-backend`

- `Name`: `Select Trusted CLI Backend`
- `Surface`: `settings-or-extensions`
- `Trigger`: 用户尝试切换到 `trusted-cli`
- `Preconditions`: backend picker 可见
- `Immediate Result`: 先经过 trusted gate；仅在 `enabled + trusted + AGENT_CLI_PATH` 全满足时才进入 trusted backend
- `Application Entry`: `apps/web/src/components/settings_sections.rs`, `apps/web/src/components/sidebar/extensions.rs`

### `op.ai.trusted-agent.submit`

- `Name`: `Submit Trusted Agent Prompt`
- `Surface`: `chat-panel`
- `Trigger`: 用户在 trusted backend 下点击发送或按 Enter
- `Preconditions`: 当前 backend 已切到 `trusted-cli`，prompt 非空
- `Immediate Result`: 通过 trusted gate 后才允许发起 bridge 请求；失败时必须回退 `native`
- `Application Entry`: `apps/web/src/components/chat/actions_send.rs`, `apps/cli/src/server/handlers/plugin.rs`

### `op.ai.trusted-agent.receive-disabled`

- `Name`: `Receive Trusted Agent Disabled State`
- `Surface`: `settings-or-chat`
- `Trigger`: trusted 条件不满足，或系统主动执行 fallback
- `Preconditions`: 用户尝试选择 / 使用 `trusted-cli`
- `Immediate Result`: 前端保持或回退到 `native`，并显示明确原因
- `Application Entry`: `apps/web/src/hooks/use_ai_backend.rs`, `apps/web/src/components/settings_sections.rs`, `apps/web/src/components/sidebar/extensions.rs`, `apps/cli/src/server/agent_bridge.rs`

### `op.ai.trusted-agent.receive-stream`

- `Name`: `Receive Trusted Agent Stream`
- `Surface`: `chat-panel`
- `Trigger`: trusted bridge 返回流式 chunk
- `Preconditions`: `op.ai.trusted-agent.submit` 已成功越过 trusted gate
- `Immediate Result`: assistant 消息增量更新，直到 bridge 显式结束
- `Application Entry`: `apps/cli/src/server/agent_bridge/stream.rs`, `apps/web/src/hooks/use_core/effects/message_dispatch_runtime.rs`

## Notes

- 这条 flow 建模的是 `trusted-cli` 的安全边界，而不是通用 plugin runtime。
- `trusted-cli` 必须保持 default-off；它是高级部署位，不是默认 shipped backend。
- 若 `enabled / trusted / AGENT_CLI_PATH` 任一条件不满足，系统必须 fail-closed 并回退到 `native`。
- 前端 Settings 与 Extensions 共享同一 capability/fallback hook；如果当前已处于 `agent-bridge`
  但 capability probe 返回不可用，必须回退 `native` 并向 chat 写入可见原因。

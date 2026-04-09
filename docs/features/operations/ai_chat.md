# ai_chat.md - Native AI Chat 操作流示例

## Metadata

- `Flow ID`: `flow.ai.chat`
- `Domain`: `ai`
- `Related Feature Chapters`: `docs/features/10_ai_agent.md`
- `Related Acceptance Cases`: `AI-FEAT-01`

## Operations

### `op.ai.chat.open-panel`

- `Name`: `Open AI Chat Panel`
- `Surface`: `workspace-shell`
- `Trigger`: 点击 chat panel、侧栏入口，或执行等价命令
- `Preconditions`: 主界面已加载
- `Immediate Result`: chat panel 打开
- `Application Entry`: `apps/web/src/components/chat/panel.rs`, `apps/web/src/components/main_layout.rs`

### `op.ai.chat.type-message`

- `Name`: `Type AI Prompt`
- `Surface`: `chat-panel`
- `Trigger`: 用户在 chat 输入框键入消息
- `Preconditions`: panel 已打开
- `Immediate Result`: 本地 input state 更新
- `Application Entry`: `apps/web/src/components/chat/input_area.rs`

### `op.ai.chat.submit`

- `Name`: `Submit AI Prompt`
- `Surface`: `chat-panel`
- `Trigger`: 点击发送按钮或按 Enter
- `Preconditions`: prompt 非空，当前未在 streaming
- `Immediate Result`: 前端附加 user/assistant placeholder，发起 plugin call
- `Application Entry`: `apps/web/src/components/chat/actions_send.rs`, `apps/cli/src/server/handlers/plugin.rs`

### `op.ai.chat.receive-stream`

- `Name`: `Receive AI Stream`
- `Surface`: `chat-panel`
- `Trigger`: 服务端返回 `ChatChunk` 或 plugin response
- `Preconditions`: `op.ai.chat.submit` 已执行
- `Immediate Result`: assistant 消息增量更新，或显示明确失败
- `Application Entry`: `apps/web/src/components/chat/panel_effects.rs`, `apps/cli/src/server/ai_chat/mod.rs`, `crates/core/src/plugin/runtime/chat_stream.rs`

## Notes

- 当前图只建模 `Native AI Chat` 主线，不把 `trusted-cli` 当成默认 flow。
- AI chat 是外围辅助流，不能反向主导 repo、source control 或 authority 主链。

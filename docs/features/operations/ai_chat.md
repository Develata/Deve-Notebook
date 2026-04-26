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

### `op.ai.chat.switch-native-mode`

- `Name`: `Switch Native Chat Mode`
- `Surface`: `chat-panel`
- `Trigger`: 输入 `/plan`、`/build` 或 `/agents`
- `Preconditions`: panel 已打开，当前未在 streaming
- `Immediate Result`: 只切换本地 Native `PLAN` / `BUILD` 会话模式，不切换 backend，不发起 plugin call
- `Application Entry`: `apps/web/src/components/chat/slash_commands.rs`, `apps/web/src/components/chat/actions_send.rs`

### `op.ai.chat.apply-controlled-markdown`

- `Name`: `Apply Controlled Markdown Edit`
- `Surface`: `chat-panel`
- `Trigger`: 在 BUILD 模式下点击 assistant code block 的 `Apply`
- `Preconditions`: 当前 session mode 为 `BUILD`，当前文档可写，repo writer ready，local scope nonce 稳定
- `Immediate Result`: 通过现有 `ClientMessage::Edit` 管道把 code block 作为受控 Markdown delta 追加到当前文档
- `Application Entry`: `apps/web/src/components/chat/message_list.rs`, `apps/web/src/components/chat/actions_apply.rs`

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
- `ai_mode` / backend 仍表示 `Native AI` 或 `Trusted CLI`；`PLAN` / `BUILD` 是独立的
  Native 会话模式，当前随后续 prompt 作为 `chat_mode` context 传入。
- `/build` 本身不直接改写 Markdown；任何 Markdown 写入必须走后续受控 apply / edit 路径。
- `Apply` 只在 BUILD 模式下为 assistant code block 显示；PLAN 模式不得暴露可写入口。

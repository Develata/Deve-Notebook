# Native AI positive smoke - 2026-05-12

## Scope

- 唯一真源：`docs/plan/10_ai_agent.md#native-ai-chat-runtime`
- 验收目标：不依赖真实外部 API key，使用本地 OpenAI-compatible SSE mock 验证 Native AI Chat 正向链路。
- 覆盖链路：Web Chat UI → `/api/ai/backend-capabilities` → `PluginCall ai-chat::chat` → Rhai `ai-chat` → server `AiChatStreamHandler` → SSE provider → `ChatChunk` / `PluginResponse` → UI 完成 loading。

## Finding

- 首轮 Chrome smoke 失败：`plugins/ai-chat/main.rhai` 的 `SYSTEM_PROMPT` 在 `chat -> build_system_prompt` 嵌套调用路径中不可见，导致正向 provider 分支抛出 `Variable not found: SYSTEM_PROMPT`。
- 修复：将常量改为 `system_prompt_base()` 函数返回值，避免 Rhai 常量作用域在嵌套调用路径中的运行时差异。
- 另一个测试环境问题：宿主代理变量会劫持 `127.0.0.1` mock 请求并返回 502。最终 smoke 在 serve 进程中清空 proxy env，并设置 `NO_PROXY=127.0.0.1,localhost`。

## Browser Smoke

- Mock provider：本地 `POST /v1/chat/completions`，返回 OpenAI-compatible `text/event-stream`。
- Deve server：`serve --dev --port 3013`
- 数据根：`/tmp/deve-native-ai-positive-1778599647`
- Provider env：
  - `AI_BASE_URL=http://127.0.0.1:19081/v1`
  - `AI_API_KEY=deve-smoke-key`
  - `AI_MODEL=deve-smoke`
- Chrome 操作：
  - 登录 `admin/admin`
  - 新建 `Untitled.md`
  - Chat 输入 `Summarize this markdown file`
  - 点击发送

## Result

- UI 显示：`Mock native AI response: current markdown context received.`
- `Plugin runtime error`：未出现。
- `No AI API key`：未出现。
- Loading：请求完成后停止。
- Console：无 error / warn；仅初始化、WS、OpenDoc、Snapshot 日志。
- Network：`/api/ai/backend-capabilities` 返回 200。

## Regression Guards

- `plugins/ai-chat/main.rhai` 使用 `system_prompt_base()`。
- `crates/core/tests/ai_chat_plugin_test.rs` 新增 `test_chat_with_api_key_reaches_stream_bridge`，保证有 dummy key 时插件能走到 stream bridge，而不是在 prompt 构造处失败。
- `docs/acceptance-cases/10_plugins.md` 的 `AI-001` 新增 `cargo test -p deve_core --test ai_chat_plugin_test -- --nocapture`。
- `scripts/check-ai-baseline.sh` 绑定 `system_prompt_base` 与正向 stream bridge 测试名。

## Verification

- `cargo test -p deve_core --test ai_chat_plugin_test -- --nocapture`
- `scripts/check-ai-baseline.sh`

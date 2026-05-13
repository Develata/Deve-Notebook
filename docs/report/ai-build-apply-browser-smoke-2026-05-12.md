# AI BUILD Apply browser smoke - 2026-05-12

## Scope

- 唯一真源：`docs/plan/10_ai_agent.md#native-ai-chat-runtime`
- 验收目标：闭合 `AI-003` 的真实浏览器链路。
- 覆盖链路：`/build` → assistant fenced code block → `Apply` → `ClientMessage::Edit` → 当前 Markdown 变化。

## Finding

- 首轮 Chrome smoke 暴露缺陷：`Apply` 会发送 WS edit，server 也会持久化到 vault，但当前 CodeMirror 视图不变。
- 根因：`Apply` 是程序化 local edit，不经过用户键入产生的 CodeMirror delta；既有 pending overlay 假定本地视图已经应用该 op。
- 修复：`apps/web/src/components/chat/actions/apply.rs` 在登记 pending 与发送 `ClientMessage::Edit` 前，将同一个 `Op` 应用到本地 CodeMirror 视图。
- 复审修复：本地 Apply 后调用 `syncEditorStateToRust`，复用编辑器空 delta 同步路径刷新 Rust-side `content` signal、stats 与 Outline。

## Browser Smoke

- Web assets：`scripts/smoke-web-release-build.sh`
- Mock provider：本地 `POST /v1/chat/completions`，返回 OpenAI-compatible `text/event-stream`，内容包含一个 `md` fenced code block。
- Deve server：`serve --dev --port 3016`
- 数据根：`/tmp/deve-ai-build-apply-20260512-final`
- Provider env：
  - `AI_BASE_URL=http://127.0.0.1:19082/v1`
  - `AI_API_KEY=deve-smoke-key`
  - `AI_MODEL=deve-smoke`
- Chrome 操作：
  - 新建 `Untitled.md`
  - 输入并提交 `/build`
  - 输入并提交 `Return a small markdown patch as one fenced md code block.`
  - 点击 assistant code block 的 `Apply`

## Result

- `/build` 后 `slashCommandSentWs = 0`，未发 plugin call。
- assistant response 显示一个 `Apply` 按钮。
- 点击前编辑器内容为空。
- 点击后编辑器内容为：
  ```md

  AI BUILD controlled apply smoke line
  ```
- UI stats 同步为 `chars=38 / words=6 / lines=2`。
- vault 文件 `/tmp/deve-ai-build-apply-20260512-final/vault/default/Untitled.md` 与编辑器内容一致。
- Network：`/api/ai/backend-capabilities` 返回 200。
- Console：当前页面无 error / warn。
- Mock provider request system prompt 明确包含 `do not execute workspace, source-control, shell, MCP, or skill actions`。

## Follow-up Smoke

- 日期：2026-05-13
- 目标：验证本地 Apply 后 Rust-side `content` signal、stats 与 Outline 同步。
- Deve server：`serve --dev --port 3019`
- 数据根：`/tmp/deve-ai-build-apply-sync-20260513c`
- Provider env：`AI_BASE_URL=http://127.0.0.1:19084/v1`，并设置 `NO_PROXY=127.0.0.1,localhost`。
- `/build` 后 `sendDelta = 0`，仅切换 BUILD session mode。
- prompt 后 `sendDelta = 1`，mock provider 收到一次 `POST /v1/chat/completions`。
- 点击本地化按钮 `应用` 前编辑器为空；点击后编辑器内容为：
  ```md
  # AI BUILD Outline Smoke

  AI BUILD controlled apply smoke line
  ```
- vault 文件 `/tmp/deve-ai-build-apply-sync-20260513c/vault/default/Untitled.md` 与编辑器内容一致。
- Outline 出现 `title="AI BUILD Outline Smoke"` 条目。
- UI stats 同步为 `words=11 / lines=3 / chars=63`。
- Console：当前页面无 error / warn。

## Regression Guards

- `cargo test -p deve_web chat_apply -- --nocapture`
- `scripts/check-ai-baseline.sh`
- `scripts/check-rendering-baseline.sh`
- `scripts/smoke-web-release-build.sh`

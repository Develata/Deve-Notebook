# AI Slash Command Browser Smoke - 2026-05-13

本报告记录 `/plan` 与 `/agents` 浏览器点验。`docs/plan/` 仍是唯一权威；本文件只记录当前实现的验证结果。

## Scope

- `docs/plan/10_ai_agent.md#native-ai-chat-runtime`
- `docs/features/operations/ai_chat.md`
- `docs/acceptance-cases/10_plugins.md` AI-002 / AI-004

## Environment

- Backend: `DEVE_LEDGER_DIR=/tmp/deve-ai-slash-smoke-20260513-LOjOOj/ledger DEVE_VAULT_PATH=/tmp/deve-ai-slash-smoke-20260513-LOjOOj/vault DEVE_STATIC_DIR=apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 32126`
- Browser URL: `http://127.0.0.1:32126/`
- Data root: `/tmp/deve-ai-slash-smoke-20260513-LOjOOj`
- Chrome MCP viewport: mobile emulation `375x812`
- Auth: development defaults
- AI provider: intentionally not configured; slash commands must not call provider or plugin.

## Results

Passed:

- Mobile AI Chat opened with backend label `原生 AI` and initial native session mode `计划`.
- Submitting `/plan` appended the localized assistant notice:
  - `已切换到计划模式。我会保持只读并避免工具调用。`
- Submitting `/agents` switched from PLAN to BUILD and appended:
  - `已切换到执行模式。Markdown 修改仍需走受控应用路径。`
- Submitting `/agents` again switched from BUILD back to PLAN and appended the PLAN notice again.
- Visible mode sequence matched AI-004: PLAN -> BUILD -> PLAN.
- Chat input remained mounted, enabled, and empty after the slash-command sequence.
- Slash commands did not append user prompt bubbles or assistant placeholders.
- Current-page network had no `/api/ai/backend-capabilities` request during the slash-command sequence.
- Server log after opening Chat contained only connection, repo switch, and `SyncHello` records; no plugin/provider handling log appeared.
- Browser console contained no `error` or `warn` entries.
- A single 14-byte background WS frame was observed during the sequence; no backend-capability fetch, ChatChunk, plugin response, or AI provider path was observed.

## Boundary

- `/plan`, `/build`, and `/agents` remain local Native session-mode switches.
- Slash commands do not switch `native` / `trusted-cli` backend.
- Slash commands do not implicitly start Trusted CLI, tools, Skills, MCP, source-control writes, or provider calls.

## Verification

已运行：

- `bash scripts/check-ai-baseline.sh`
- `cargo test -p deve_web slash_commands -- --nocapture`
- `cargo test -p deve_web slash_commands_preserve_backend_mode -- --nocapture`
- `cargo test -p deve_web chat_apply -- --nocapture`
- `cargo test -p deve_cli ai_chat -- --nocapture`
- Chrome MCP browser smoke as described above

结果：

- AI baseline guard: pass.
- Slash command tests: pass, 4 tests.
- Slash backend-preservation test: pass, 1 test.
- Chat Apply tests: pass, 5 tests.
- Server AI Chat fail-closed/tool rejection tests: pass, 15 tests.
- Browser slash command smoke: pass.

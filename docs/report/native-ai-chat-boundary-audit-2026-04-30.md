# Native AI Chat Boundary Audit - 2026-04-30

## Scope

Audited current Native AI Chat against `docs/plan/10_ai_agent.md`, `docs/features/10_ai_agent.md`, and the active queue in `docs/report/next-tasks.md`.

## Closed In This Batch

- Chat `PluginResponse` completion now stops the matching assistant placeholder from loading forever. Synchronous text responses such as missing API key are rendered into an empty assistant message, while streamed content is not duplicated.
- Product backend names are now `native` / `trusted-cli` in Web state. Runtime plugin ids `ai-chat` / `agent-bridge` are reached through an explicit conversion layer.
- Native chat now sends bounded prior user/assistant history to the compatibility plugin for multi-turn context; plugins still only receive context explicitly passed by the frontend.
- Provider `tool_calls` remain fail-closed and no longer emit a normal finish chunk before the rejection.
- Trusted CLI messaging now matches policy: `AGENT_CLI_PATH` must be an existing absolute executable path, not an implicit PATH lookup.
- Local AGENTS docs and core chat-stream comments now describe `ToolCalls` / `ai_chat_stream_with_tools` as reserved/fail-closed compatibility, not a current tool loop.

## Remaining Non-Blockers

- Native Chat still routes through the bundled `plugins/ai-chat` compatibility plugin and `ClientMessage::PluginCall`. This is acceptable for the current P1 boundary because the public method is limited to `chat`, but a future first-party `ClientMessage::AiChat` path would make the architecture cleaner.
- Acceptance case `AI-001` still cannot prove semantic model output without a fake provider/browser E2E harness. Current automated checks verify payload boundaries, rejection behavior, and UI completion semantics.

## Verification

- `cargo test -p deve_web plugin_text_response -- --nocapture`
- `cargo test -p deve_web bounded_history -- --nocapture`
- `cargo test -p deve_web maps_product_backend_names_to_runtime_plugin_ids -- --nocapture`
- `cargo test -p deve_web slash_commands -- --nocapture`
- `cargo test -p deve_cli ai_chat -- --nocapture`
- `cargo test -p deve_cli agent_bridge -- --nocapture`
- `cargo test -p deve_core --test ai_chat_plugin_test -- --nocapture`
- `cargo fmt --check`
- `scripts/check-ai-baseline.sh`
- `scripts/plan-coverage.sh`
- `git diff --check`

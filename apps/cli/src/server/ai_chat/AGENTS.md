<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# ai_chat

## Purpose

AI chat streaming integration. Handles SSE-based streaming from AI providers, configuration management, and message type definitions for the chat interface.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Module entry and stream handler initialization |
| `config.rs` | AI provider configuration (API keys, endpoints) |
| `stream.rs` | SSE stream handling — proxies AI provider responses |
| `sse_parser.rs` | Server-Sent Events parser |
| `types.rs` | Chat message and response type definitions |

## For AI Agents

### Working In This Directory

- See `docs/ai-chat-streaming.md` and `10_ai_agent.md` in deve-note plan.
- Stream handler is initialized once at server startup.

<!-- MANUAL: -->

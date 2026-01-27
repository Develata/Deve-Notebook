# AI Chat Streaming Bridge Design

## Overview
This document specifies the AI chat streaming bridge that connects the Rhai
plugin runtime with the server-side OpenAI-compatible SSE implementation.

## Constraints
- Target environment: 768MB VPS.
- Core crate must remain runtime-agnostic (no tokio dependency).
- Streaming must remain non-blocking for async WebSocket handlers.

## Architecture
1. **Core Bridge** (`crates/core/src/plugin/runtime/chat_stream.rs`)
   - Defines `ChatStreamHandler` and `ChatStreamSink`.
   - Stores a global handler (OnceLock).
   - Uses a thread-local sink scope for per-request routing.

2. **Server Runtime** (`apps/cli/src/server/ai_chat.rs`)
   - Implements `ChatStreamHandler` using `reqwest-eventsource`.
   - Streams SSE deltas into `ServerMessage::ChatChunk` via the sink.

3. **Plugin Call Path** (`apps/cli/src/server/handlers/plugin.rs`)
   - Wraps plugin calls with `ChatStreamScope`.
   - Ensures streaming chunks are routed to the correct client.

4. **Host Function** (`crates/core/src/plugin/runtime/host.rs`)
   - Registers `ai_chat_stream`.
   - Enforces network capability checks by domain.

## Data Flow
1. Web client invokes plugin call for `ai-chat::chat`.
2. Server sets a `ChatStreamScope` and calls `plugin.call`.
3. Rhai script calls `ai_chat_stream` with `req_id`, config, and history.
4. Core bridge routes the request to the server handler.
5. Handler performs SSE stream and emits `ChatChunk` updates.
6. Client assembles deltas into the final assistant message.

## Error Handling
- Missing handler or sink yields a clear runtime error to the plugin.
- SSE decode errors bubble up as plugin runtime errors.
- Client streaming ends when `finish_reason` is received.

## Security
- Network requests are domain-validated against plugin capabilities.
- The handler never exposes raw HTTP response data to plugins.

## Low-Resource Notes
- The bridge avoids new runtime dependencies in `deve_core`.
- Streaming is handled in the CLI where tokio is already required.

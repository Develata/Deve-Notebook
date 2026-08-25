# AI Chat Streaming Bridge Design

## Overview
This document specifies the AI chat streaming bridge that connects the Rhai
compatibility runtime with the server-owned multi-provider SSE implementation.

## Constraints
- Target environment: 768MB VPS.
- Core crate must remain runtime-agnostic (no tokio dependency).
- Streaming must remain non-blocking for async WebSocket handlers.

## Architecture
1. **Core Bridge** (`crates/core/src/plugin/runtime/chat_stream.rs`)
   - Defines `ChatStreamHandler` and `ChatStreamSink`.
   - Stores a global handler (OnceLock).
   - Uses a thread-local sink scope for per-request routing.

2. **Server Runtime** (`apps/cli/src/server/ai_chat/`)
   - Owns the redacted provider settings snapshot and three protocol adapters.
   - Implements `ChatStreamHandler` using a server-owned bounded incremental SSE decoder over `reqwest` bytes.
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
3. Rhai script calls `ai_chat_stream` with `req_id` and history only; it has no provider secret or network authority.
4. Core bridge routes the request to the server handler.
5. Handler performs SSE stream and emits `ChatChunk` updates.
6. Client assembles deltas into the final assistant message.

## Error Handling
- Missing handler or sink yields a clear runtime error to the plugin.
- SSE decode errors bubble up as plugin runtime errors.
- Client streaming ends when `finish_reason` is received.
- Connect/header wait is bounded to 30 seconds, chunk idle to 30 seconds, and the entire request to 5 minutes.
- A frame is bounded to 256 KiB, total wire input to 8 MiB, and accepted answer text to 2 MiB. Admission happens before a delta is appended or forwarded.

## Security
- Provider URL and authentication are validated and owned by the server settings runtime.
- Raw API keys never cross into Rhai, Web responses, logs, or browser storage.
- External PluginCall access for bundled Native AI is `ai-chat::chat` only.
- The handler never exposes raw HTTP response data to plugins.

## Low-Resource Notes
- The bridge avoids new runtime dependencies in `deve_core`.
- Streaming is handled in the CLI where tokio is already required.
- The decoder retains only the current bounded line/event plus the bounded accepted answer; it never buffers the entire wire stream.

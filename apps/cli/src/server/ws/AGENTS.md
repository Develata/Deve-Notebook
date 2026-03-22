<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# ws

## Purpose

WebSocket connection management: message filtering, receiving client messages, sending server messages, and dispatching to appropriate handlers.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | WebSocket module entry |
| `filter.rs` | Message filtering — validates and routes incoming WS messages |
| `receive.rs` | Receives and deserializes client WebSocket messages |
| `send.rs` | Serializes and sends server WebSocket messages |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `route/` | WebSocket route handlers by domain |

## For AI Agents

### Working In This Directory

- All messages are repo-scoped — every message carries repo_id, branch, scope_nonce.
- `filter.rs` is the first validation gate for incoming messages.

<!-- MANUAL: -->

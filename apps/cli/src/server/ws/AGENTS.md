<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# ws

## Purpose

WebSocket connection management: message filtering, receiving client messages, sending server messages, and dispatching to appropriate handlers.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | WebSocket module entry |
| `filter/` | Broadcast filtering — enforces repo/branch/scope delivery and nonce stamping |
| `receive/` | Receives, validates, and deserializes client WebSocket messages |
| `send.rs` | Serializes and sends server WebSocket messages |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `filter/` | Broadcast filter facade, scope matching, outbound stamping, and regression tests |
| `receive/` | Inbound frame decoding, legacy text debug gate, rate-limit handling, and regression tests |
| `route/` | WebSocket route handlers by domain |

## For AI Agents

### Working In This Directory

- All messages are repo-scoped — every message carries repo_id, branch, scope_nonce.
- `filter/` is the first validation gate for outgoing broadcast delivery.

<!-- MANUAL: -->

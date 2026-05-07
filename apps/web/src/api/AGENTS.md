<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# api

## Purpose

WebSocket API layer. Manages the connection lifecycle, message serialization/deserialization, auth probing, exponential backoff reconnection, and writer ID tracking.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Module entry and re-exports |
| `connection.rs` | WebSocket connection management — connect, reconnect, close |
| `connection_role.rs` | Node role probing and runtime summary formatting |
| `connection_role_test.rs` | Node role probe formatting and stale-epoch unit tests |
| `incoming.rs` | Incoming ServerMessage deserialization and dispatch |
| `output.rs` | Outgoing ClientMessage serialization and sending |
| `auth_probe.rs` | Authentication status probing before connection |
| `backoff.rs` | Exponential backoff for reconnection attempts |
| `writer_id.rs` | Writer identity tracking for OT conflict resolution |

## For AI Agents

### Working In This Directory

- All messages are repo-scoped — must carry repo_id, branch, scope_nonce.
- Connection uses exponential backoff on failure.

<!-- MANUAL: -->

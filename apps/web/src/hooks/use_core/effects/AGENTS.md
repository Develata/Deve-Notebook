<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# effects

## Purpose

Server message effect handlers. Processes incoming WebSocket messages and updates reactive state: handshake flow, message dispatch, protocol handling, scope management, shadow branches, sync state, and source control updates.

## Key Files

| File | Description |
|------|-------------|
| `handshake/` | Handshake effect — initial connection, retry, suspend, signing, and tests |
| `handshake_bootstrap.rs` | Handshake bootstrap for first connection |
| `handshake_bootstrap/` | Handshake bootstrap helpers |
| `message.rs` | Top-level message effect entry |
| `message_dispatch.rs` | Message dispatch to specific handlers |
| `message_dispatch_gate.rs` | Gate logic for message dispatch |
| `message_dispatch_protocol.rs` | Protocol message dispatch |
| `message_control.rs` | Control message handling |
| `message_protocol.rs` | Protocol message processing |
| `message_projection.rs` | Projection update handling |
| `message_refresh.rs` | Refresh message handling |
| `message_repo_scope.rs` | Repo scope message handling |
| `message_runtime.rs` | Runtime message handling |
| `message_scope.rs` | Scope message handling |
| `message_shadow.rs` | Shadow branch message handling |
| `message_sync.rs` | Sync message handling |

## For AI Agents

### Working In This Directory

- Messages are dispatched through `message_dispatch.rs` gate logic.
- Scope messages must validate scope_nonce before applying state changes.

<!-- MANUAL: -->

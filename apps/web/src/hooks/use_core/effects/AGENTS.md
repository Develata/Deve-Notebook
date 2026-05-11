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
| `message_dispatch_gate/` | Gate logic for message dispatch and tests |
| `message_dispatch_protocol.rs` | Protocol message dispatch |
| `message_control.rs` | Control message handling |
| `message_control_runtime/` | Control runtime handling and tests |
| `message_control_runtime_repo/` | Repo-scoped control runtime helpers |
| `message_protocol/` | Protocol message processing and tests |
| `message_projection/` | Projection update handling and tests |
| `message_refresh/` | Refresh message handling and tests |
| `message_repo_scope/` | Repo scope message handling and tests |
| `message_dispatch_route_projection/` | Projection/sync routing helpers |
| `message_runtime.rs` | Runtime message handling |
| `message_dispatch_runtime/` | Runtime response dispatch handling and tests |
| `message_scope/` | Scope message handling and tests |
| `message_shadow/` | Shadow branch message handling and tests |
| `message_runtime_sync/` | Runtime sync status/pending-op handling |
| `message_sync/` | Sync message handling and tests |

## For AI Agents

### Working In This Directory

- Messages are dispatched through `message_dispatch.rs` gate logic.
- Scope messages must validate scope_nonce before applying state changes.

<!-- MANUAL: -->

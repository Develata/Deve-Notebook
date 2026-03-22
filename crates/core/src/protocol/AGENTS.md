<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# protocol

## Purpose

WebSocket protocol definitions shared between server and client. Defines `ClientMessage`, `ServerMessage`, error codes, auth messages, and confirmed operation types.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Module entry and re-exports |
| `client.rs` | ClientMessage enum — all messages client can send |
| `server.rs` | ServerMessage enum — all messages server can send |
| `error.rs` | ServerErrorCode enum and error message types |
| `auth.rs` | Authentication protocol messages |
| `confirmed_op.rs` | Confirmed operation types for OT |
| `sc_path_target.rs` | Source control path target types |

## For AI Agents

### Working In This Directory

- All messages are repo-scoped with repo_id, branch, scope_nonce.
- Changes here affect both server and web client — ensure both are updated.
- See `05_network.md` in deve-note plan for protocol design.

<!-- MANUAL: -->

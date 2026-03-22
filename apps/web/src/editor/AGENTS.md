<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# editor

## Purpose

Document editor integration. Manages OT operations, editor handshake with server, content sync, playback, FFI bridge to JavaScript editor, and prefetch optimization.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Editor module entry |
| `hook.rs` | Editor reactive hook — main editor lifecycle |
| `ffi.rs` | FFI bridge to JavaScript editor (CodeMirror/etc.) |
| `delta_input.rs` | Delta input processing from editor |
| `buffered_ops.rs` | OT operation buffering |
| `op_id.rs` | Operation ID generation |
| `handshake_reset.rs` | Editor handshake reset on reconnect |
| `message_effect.rs` | Server message effect on editor state |
| `open_scope.rs` | Document open scope tracking |
| `playback.rs` | History playback in editor |
| `prefetch.rs` | Document content prefetch |
| `request_key.rs` | E2E encryption key request |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `sync/` | Editor sync — snapshot, history, encryption |

<!-- MANUAL: -->

<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# sync

## Purpose

Sync engine handlers at the WebSocket boundary. Manages the sync lifecycle: hello handshake, scope validation, snapshot exchange, transfer operations, and writer readiness.

## Key Files

| File | Description |
|------|-------------|
| `hello/mod.rs` | Sync hello handshake — initiates peer connection |
| `hello/response.rs` | Sync hello response signing and emission |
| `hello/outbound.rs` | Non-browser SyncHello follow-up request/push emission |
| `hello/scope.rs` | Scope validation during sync hello |
| `hello/scope/browser.rs` | Browser-specific SyncHello scope validation |
| `engine.rs` | Sync engine orchestration |
| `snapshot.rs` | Snapshot exchange during sync |
| `transfer.rs` | Data transfer operations |
| `writer.rs` | Writer readiness notification |
| `guard.rs` | Sync concurrency guard |
| `cleanup.rs` | Cleanup stale sync state |
| `errors.rs` | Sync-specific error types |

## For AI Agents

### Working In This Directory

- Sync hello must validate scope nonce — stale nonces must be rejected.
- Transfer operations use the PersistGuard to prevent watcher storms.

<!-- MANUAL: -->

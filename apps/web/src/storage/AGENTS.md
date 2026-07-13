<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# storage

## Purpose

Browser-side storage abstraction. Provides localStorage preferences, IndexedDB via JS bridge, and identity persistence.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Storage module entry |
| `capability.rs` | Typed browser identity capability facts, fail-closed blockers, and log-only diagnostics |
| `prefs.rs` | User preferences in localStorage |
| `js_bridge.rs` | JavaScript/WASM bridge for IndexedDB |
| `identity.rs` | Client identity persistence |

<!-- MANUAL: -->

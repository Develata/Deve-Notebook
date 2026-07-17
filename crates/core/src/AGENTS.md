<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# src

## Purpose

Root source of the core library. Declares all public modules and gates backend-only code behind `#[cfg(not(target_arch = "wasm32"))]`.

## Key Files

| File | Description |
|------|-------------|
| `lib.rs` | Module declarations and crate-level docs |
| `config.rs` | Configuration loading (env/config.toml) with profile support |
| `error.rs` | Unified error types using thiserror |
| `models/mod.rs` | Core data types (re-exports from models/) |
| `state.rs` | Document state (re-exports from state/) |
| `vfs.rs` | Virtual filesystem operations (backend-only) |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `remote_projection/` | Remote Projection transport model used by Projection Backup surfaces |
| `sync/watcher/` | Owned filesystem-ingestion handles, backend adapters, normalization, and typed runtime diagnostics |
| `context/` | Context engine (tree context) |
| `ledger/` | Ledger storage — the heart of the system |
| `models/` | Data model types and serialization |
| `plugin/` | Plugin system and Rhai runtime |
| `protocol/` | WebSocket message definitions |
| `search/` | Full-text search (feature-gated) |
| `security/` | Cryptography, auth, permissions |
| `skill/` | Skill system |
| `source_control/` | Git-like source control logic |
| `state/` | Document state and OT |
| `sync/` | Sync engine and P2P reconciliation |
| `tree/` | Document tree management |
| `utils/` | Shared utilities |

## For AI Agents

### Working In This Directory

- Backend-only: `ledger`, `vfs`, `watcher`, `search`.
- Cross-platform: `models`, `protocol`, `state`, `source_control`, `sync`, `tree`, `utils`, `plugin`, `security`, `skill`, `context`.

<!-- MANUAL: -->

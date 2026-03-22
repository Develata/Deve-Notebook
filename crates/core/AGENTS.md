<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# core

## Purpose

The `deve_core` library crate containing all business logic for Deve-Notebook. Platform-agnostic core shared by both the CLI server and the WASM frontend. Implements ledger-first storage, sync engine, source control, plugin system, security, search, protocol definitions, and tree management.

## Key Files

| File | Description |
|------|-------------|
| `Cargo.toml` | Crate manifest — redb, serde, uuid, chrono, rhai, ed25519-dalek |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `src/` | All source code |

## For AI Agents

### Working In This Directory

- This is a library crate — use `thiserror` for errors, not `anyhow`.
- Backend-only modules are gated with `#[cfg(not(target_arch = "wasm32"))]`.
- Run tests: `cargo test -p deve_core`
- Run checks: `cargo check -p deve_core && cargo clippy -p deve_core --all-targets --all-features -- -D warnings`

<!-- MANUAL: -->

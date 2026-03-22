<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# cli

## Purpose
The `deve_cli` crate is the Axum-based HTTP/WebSocket server binary for Deve-Notebook. It serves as the Local Hub and Backend Server, providing CLI commands (init, scan, watch, dump, serve, export, repair, seed, verify-p2p, node-check) and an Axum server with REST API endpoints, WebSocket real-time collaboration, plugin hosting via Rhai, MCP integration, and OpenAI-compatible AI chat streaming. It depends on `deve_core` for all domain logic including the ledger, sync engine, CRDT operations, and security primitives.

## Key Files
| File | Description |
|------|-------------|
| `Cargo.toml` | Crate manifest; depends on deve_core, axum 0.7, tokio, clap, reqwest, rhai, redb |

## Subdirectories
| Directory | Purpose |
|-----------|---------|
| `src/` | All Rust source code for the CLI binary and server |

## For AI Agents

### Working In This Directory
- This crate depends on `deve_core` for all business logic.
- Server binds to `0.0.0.0:{port}` (default 3001).
- The `search` feature flag gates tantivy-based full-text search via `deve_core/search`.
- Plugin system uses Rhai scripting engine; plugins load from a `plugins/` directory at runtime.
- Auth uses JWT tokens in HttpOnly cookies with brute-force protection and per-IP rate limiting.
- All fail-closed patterns (poisoned locks, missing metadata, broken config) must be preserved.
- Run tests: `cargo test -p deve_cli`
- Run checks: `cargo check -p deve_cli && cargo clippy -p deve_cli --all-targets --all-features -- -D warnings`

<!-- MANUAL: -->

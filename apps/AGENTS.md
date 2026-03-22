<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# apps

## Purpose

Application binaries for Deve-Notebook. Contains the CLI server (`cli`) providing the Axum WebSocket/HTTP backend, and the Web frontend (`web`) built with Leptos as a WASM single-page application.

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `cli/` | Axum-based server, CLI commands, WebSocket handlers (see `cli/AGENTS.md`) |
| `web/` | Leptos WASM frontend — editor, sidebar, source control UI (see `web/AGENTS.md`) |

## For AI Agents

### Working In This Directory

- `cli` and `web` are separate workspace members with their own `Cargo.toml`.
- Both depend on `deve_core` for business logic.
- The web app connects to the CLI server via WebSocket for real-time sync.
- `trunk serve` proxies API requests to the CLI server during development.

### Testing Requirements

- CLI: `cargo test --package deve_cli`
- Web: Build with `trunk build`; manual testing in browser.

<!-- MANUAL: -->

<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# apps

## Purpose

Application binaries for Deve-Notebook. Contains the CLI server (`cli`) providing the Axum WebSocket/HTTP backend, the Web frontend (`web`) built with Leptos as a WASM single-page application, and native shell skeleton crates.

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `cli/` | Axum-based server, CLI commands, WebSocket handlers (see `cli/AGENTS.md`) |
| `desktop/` | Minimal desktop native shell skeleton for endpoint/session/bootstrap contracts |
| `mobile/` | Minimal mobile native shell skeleton for lifecycle/reprobe/bootstrap contracts |
| `web/` | Leptos WASM frontend — editor, sidebar, source control UI (see `web/AGENTS.md`) |

## For AI Agents

### Working In This Directory

- `cli`, `desktop`, `mobile`, and `web` are separate workspace members with their own `Cargo.toml`.
- Application crates depend on `deve_core` for business logic and shared contracts.
- The web app connects to the CLI server via WebSocket for real-time sync.
- Native shell crates must not write ledger/vault/source-control/search authority directly.
- `trunk serve` proxies API requests to the CLI server during development.

### Testing Requirements

- CLI: `cargo test --package deve_cli`
- Desktop shell: `cargo test --package deve_desktop`
- Mobile shell: `cargo test --package deve_mobile`
- Web: Build with `trunk build`; manual testing in browser.

<!-- MANUAL: -->

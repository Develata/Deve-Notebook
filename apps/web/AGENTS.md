<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# web

## Purpose

Leptos/WASM frontend for Deve-Notebook. A repo-scoped thin client that connects to the CLI server via WebSocket for real-time collaboration. Renders in the browser as a single-page application with reactive signals.

## Key Files

| File | Description |
|------|-------------|
| `Cargo.toml` | Crate manifest — Leptos, wasm-bindgen, web-sys dependencies |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `src/` | All source code |

## For AI Agents

### Working In This Directory

- This is a WASM target — `#[cfg(target_arch = "wasm32")]` applies.
- Uses Leptos reactive signals (`RwSignal`, `Signal`, `Effect`).
- Communicates with server exclusively via WebSocket.
- Run checks: `cargo check -p deve_web`

<!-- MANUAL: -->

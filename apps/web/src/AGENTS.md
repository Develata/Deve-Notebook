<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# src

## Purpose

Root source directory for the web frontend. Contains the app entry point, API layer, UI components, editor integration, reactive hooks, i18n, keyboard shortcuts, local storage, and utilities.

## Key Files

| File | Description |
|------|-------------|
| `main.rs` | WASM entry point — mounts the Leptos app |
| `app.rs` | Root App component — auth state, locale context, layout routing |
| `app/` | Root App helper modules |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `api/` | WebSocket connection, message send/receive, auth probing |
| `components/` | All UI components organized by feature |
| `editor/` | Document editor — OT ops, handshake, sync, playback |
| `hooks/` | Reactive hooks — layout, outline, core state management |
| `i18n/` | Internationalization strings by feature |
| `shortcuts/` | Keyboard shortcut system |
| `storage/` | Browser local storage and IndexedDB bridge |
| `utils/` | Shared utility functions |

## For AI Agents

### Working In This Directory

- All server communication goes through `api/` — never make direct HTTP calls from components.
- Components use Leptos `#[component]` macro and reactive signals.
- State flows: Server -> WebSocket -> `hooks/use_core/` -> Components.

<!-- MANUAL: -->

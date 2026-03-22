<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# route

## Purpose

WebSocket route handlers that dispatch messages to the appropriate handler modules by domain (docs, source control, merge, scope guard).

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Route module declarations |
| `core.rs` | Core message routing logic |
| `docs.rs` | Document-related WS message routing |
| `source_control.rs` | Source control WS message routing |
| `merge.rs` | Merge WS message routing |
| `scope_guard.rs` | Scope validation guard for all WS routes |

## For AI Agents

### Working In This Directory

- `scope_guard.rs` enforces that every WS message has valid scope before processing.
- Adding new WS message types requires updating both the route and the handler.

<!-- MANUAL: -->

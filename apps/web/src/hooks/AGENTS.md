<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# hooks

## Purpose

Reactive hooks for the web frontend. Includes layout hooks, outline management, and the critical `use_core` hook that manages all server communication state.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Hook module declarations |
| `use_layout.rs` | Layout state hook (sidebar, panels) |
| `use_layout/` | Layout resize and persistence helpers |
| `use_outline.rs` | Document outline hook |
| `use_ctrl_key.rs` | Ctrl/Cmd key detection hook |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `use_core/` | Core application state — the main state management hub |

<!-- MANUAL: -->

<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# runtime

## Purpose

Plugin execution runtime built on the Rhai scripting engine. Manages chat streaming, tool registration, and provider abstractions.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | PluginRuntime trait and module entry |
| `rhai_v1.rs` | Rhai v1 engine implementation |
| `provider.rs` | Plugin provider abstraction |
| `chat_stream.rs` | Chat streaming support for plugins |
| `tools.rs` | Tool registration and execution |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `host/` | Host API functions exposed to plugin scripts |

<!-- MANUAL: -->

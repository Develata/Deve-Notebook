<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# ai-chat

## Purpose

Built-in Native AI chat compatibility plugin providing streaming LLM conversation without default tool-use. It must stay read-first unless a future plan explicitly opts in tools.

## Key Files

| File | Description |
|------|-------------|
| `manifest.json` | Plugin metadata — name, version, entry point, tool declarations |
| `main.rhai` | Chat entry point — handles message routing and streaming |

## For AI Agents

### Working In This Directory

- This is Rhai code, not Rust. Rhai syntax resembles JavaScript with some Rust influences.
- The plugin communicates with the host via registered API functions (see `crates/core/src/plugin/runtime/host/`), but default Native AI must not expose file, source-control, MCP, or skill tools.
- Public PluginCall access is `chat` only; helper/config/tool functions must remain server-side/internal.
- Streaming responses use the chat stream API defined in the plugin runtime.

### Testing Requirements

```bash
cargo test --package deve_core --test plugin_test -- --nocapture
```

<!-- MANUAL: -->

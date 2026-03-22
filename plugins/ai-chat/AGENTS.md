<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# ai-chat

## Purpose

Built-in AI chat plugin providing streaming LLM conversation with tool-use support. Integrates with the plugin host API to access note content, git operations, and search.

## Key Files

| File | Description |
|------|-------------|
| `manifest.json` | Plugin metadata — name, version, entry point, tool declarations |
| `main.rhai` | Chat entry point — handles message routing and streaming |
| `tools.rhai` | Tool function definitions callable by the AI model |

## For AI Agents

### Working In This Directory

- This is Rhai code, not Rust. Rhai syntax resembles JavaScript with some Rust influences.
- The plugin communicates with the host via registered API functions (see `crates/core/src/plugin/runtime/host/`).
- Streaming responses use the chat stream API defined in the plugin runtime.

### Testing Requirements

```bash
cargo test --package deve_core --test plugin_test -- --nocapture
```

<!-- MANUAL: -->

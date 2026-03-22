<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# plugins

## Purpose

Built-in Rhai script plugins for Deve-Notebook. Plugins extend notebook functionality through a sandboxed scripting runtime with access to the note host API. Each plugin is a directory containing a `manifest.json` and Rhai script files.

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `ai-chat/` | AI chat integration plugin — streaming chat with tool support |

## For AI Agents

### Working In This Directory

- Plugins use Rhai scripting language (not Rust).
- Each plugin must have a `manifest.json` declaring its entry point and capabilities.
- The plugin runtime is defined in `crates/core/src/plugin/`.
- Host API bindings are in `crates/core/src/plugin/runtime/host/`.

### Testing Requirements

```bash
cargo test --package deve_core --test plugin_test -- --nocapture
```

### Common Patterns

- `manifest.json` declares plugin metadata, entry scripts, and tool definitions.
- `main.rhai` is the entry point; `tools.rhai` defines callable tool functions.

<!-- MANUAL: -->

<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# plugin

## Purpose

Plugin system: manifest parsing, plugin loading, and runtime orchestration.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Plugin module entry and trait definitions |
| `manifest.rs` | Plugin manifest (manifest.json) parsing |
| `loader.rs` | Plugin discovery and loading from filesystem |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `runtime/` | Plugin execution runtime (Rhai engine) |

## For AI Agents

### Working In This Directory

- See `17_plugins.md` in deve-note plan for plugin design.
- Plugins are Rhai scripts with a `manifest.json` descriptor.

<!-- MANUAL: -->

<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# host

## Purpose

Host API functions exposed to Rhai plugin scripts. Provides file system access, git operations, note manipulation, search, skill execution, chat, and utility functions within a sandboxed path guard.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Host module entry and global state setters |
| `fs.rs` | Filesystem operations (read/write within capability-scoped paths, excluding ledger-managed projection workspaces) |
| `git.rs` | Git operations for plugins |
| `git/target.rs` | Git target resolution |
| `note.rs` | Note manipulation API |
| `search.rs` | Search API for plugins |
| `search/` | Search host helpers and tests |
| `chat.rs` | Chat API for plugins |
| `skill.rs` | Skill execution API |
| `path_guard.rs` | Path guard — constrains plugin file access around ledger-managed projection workspaces |
| `util.rs` | Utility functions |

## For AI Agents

### Working In This Directory

- `path_guard.rs` is security-critical — ensures plugins cannot bypass ledger-managed projection workspace boundaries.
- Host functions are registered into the Rhai engine scope.

<!-- MANUAL: -->

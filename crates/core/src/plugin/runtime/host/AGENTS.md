<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# host

## Purpose

Host API functions exposed to Rhai plugin scripts. Provides file system access, git operations, note manipulation, search, skill execution, chat, and utility functions within a sandboxed path guard.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Host module entry and global state setters |
| `fs.rs` | Filesystem operations (read/write within vault) |
| `git.rs` | Git operations for plugins |
| `git_target.rs` | Git target resolution |
| `note.rs` | Note manipulation API |
| `search.rs` | Search API for plugins |
| `chat.rs` | Chat API for plugins |
| `skill.rs` | Skill execution API |
| `path_guard.rs` | Path guard — constrains plugin file access to vault |
| `util.rs` | Utility functions |

## For AI Agents

### Working In This Directory

- `path_guard.rs` is security-critical — ensures plugins cannot escape the vault directory.
- Host functions are registered into the Rhai engine scope.

<!-- MANUAL: -->

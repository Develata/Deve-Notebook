<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# source_control

## Purpose

Git-like source control logic: change detection, staging/unstaging, commit diff, line diff, conflict detection, and pending filesystem state tracking. Platform-agnostic — used by both server and client.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Module entry and re-exports |
| `api.rs` | Source control API trait |
| `changes.rs` | Change detection (modified, added, deleted) |
| `staging/mod.rs` | Staging area management |
| `staging/index.rs` | Staging index operations |
| `staging/query.rs` | Staging query helpers |
| `staging/target.rs` | Staging target resolution |
| `commits/mod.rs` | Commit history |
| `commits/repair.rs` | Commit table repair |
| `commit_diff.rs` | Commit diff computation |
| `commit_diff_paths.rs` | Commit diff path resolution |
| `diff.rs` | Working directory diff |
| `line_diff.rs` | Line-level diff computation |
| `conflict.rs` | Merge conflict types |
| `types.rs` | Source control type definitions |
| `pending_fs/mod.rs` | Pending filesystem state |
| `pending_fs/index.rs` | Pending FS index |
| `pending_fs/mutation.rs` | Pending FS mutation transactions |
| `pending_fs/query.rs` | Pending FS queries |
| `pending_fs/target.rs` | Pending FS target resolution |

## For AI Agents

### Working In This Directory

- See `07_diff_logic.md` in deve-note plan for diff/SC design.
- Staging uses a dedicated index, separate from the pending FS state.
- Target resolution must use UUID-first approach.

<!-- MANUAL: -->

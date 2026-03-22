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
| `staging.rs` | Staging area management |
| `staging_index.rs` | Staging index operations |
| `staging_query.rs` | Staging query helpers |
| `staging_target.rs` | Staging target resolution |
| `commits.rs` | Commit history |
| `commit_diff.rs` | Commit diff computation |
| `commit_diff_paths.rs` | Commit diff path resolution |
| `diff.rs` | Working directory diff |
| `line_diff.rs` | Line-level diff computation |
| `conflict.rs` | Merge conflict types |
| `types.rs` | Source control type definitions |
| `pending_fs.rs` | Pending filesystem state |
| `pending_fs_index.rs` | Pending FS index |
| `pending_fs_query.rs` | Pending FS queries |
| `pending_fs_target.rs` | Pending FS target resolution |

## For AI Agents

### Working In This Directory

- See `07_diff_logic.md` in deve-note plan for diff/SC design.
- Staging uses a dedicated index, separate from the pending FS state.
- Target resolution must use UUID-first approach.

<!-- MANUAL: -->

<!-- Parent: ../AGENTS.md -->

# writeback

## Purpose

Backend-only writeback suppression infrastructure for projection and workspace writes.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Module entry and re-exports |
| `persist_guard.rs` | Repo-scoped guard used by RepoManager and SyncManager |
| `suppressor.rs` | Content/delete fingerprint suppressor for self-write watcher events |

<!-- MANUAL: -->

<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# sync

## Purpose

Sync engine and P2P reconciliation. Manages document synchronization between the server and peers, filesystem materialization, projection rebuilding, and snapshot policies.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | SyncManager struct and module entry |
| `manager_lifecycle.rs` | SyncManager construction and startup scan orchestration |
| `manager_reconcile.rs` | SyncManager document reconcile facade |
| `manager_workspace.rs` | SyncManager workspace writeback, discard, fs-event facade |
| `handler.rs` | Sync message handler |
| `materialize.rs` | Materializes ledger content to filesystem |
| `projection_io.rs` | Projection read/write |
| `projection_persistence_runtime.rs` | SyncManager projection materialize/writeback runtime facade |
| `projection_repair_runtime.rs` | SyncManager projection diagnose/rebuild/degraded health runtime facade |
| `projection_plan.rs` | Projection planning |
| `reconcile.rs` | Content reconciliation |
| `rebuild.rs` | Full projection rebuild |
| `rebuild_projection.rs` | Rebuild projection orchestration |
| `rebuild_projection_state.rs` | Rebuild state tracking |
| `repo_scoped.rs` | RepoScopedSyncEngine |
| `repo_scoped/` | Repo-scoped SyncEngine helper modules and tests |
| `scan.rs` | Projection Workspace scanning |
| `scan_file.rs` | Individual file scanning |
| `snapshot_policy.rs` | Snapshot frequency policy |
| `pending.rs` | Pending sync operations |
| `pending_content.rs` | Pending content sync |
| `pending_rename.rs` | Pending rename sync |
| `discard_pending.rs` | Discard pending changes |
| `buffer.rs` | Sync buffer |
| `dir_change.rs` | Directory change detection |
| `dir_refresh_guard.rs` | Directory refresh guard |
| `protocol.rs` | Sync protocol types |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `engine/` | Sync engine — handshake and transfer |
| `vector/` | Compatibility re-export for the shared VersionVector model |

## For AI Agents

### Working In This Directory

- **PersistGuard** lives in `writeback/` and is shared between RepoManager and SyncManager.
- `repo_scoped.rs` is the main entry point for repo-scoped sync operations.
- See `05_network.md` in deve-note plan for sync design.

<!-- MANUAL: -->

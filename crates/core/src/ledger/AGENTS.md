<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# ledger

## Purpose

The core ledger storage system — the heart of Deve-Notebook. Implements Content Facts and Structure Facts storage in Redb, document lookup, inode indexing, node operations, shadow branch management, snapshot system, source control integration, and the merge engine. This is the largest module in the codebase.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Module declarations and re-exports |
| `database.rs` | Redb database initialization and table definitions |
| `database_cache.rs` | Database query cache layer |
| `schema.rs` | Redb table schema definitions |
| `init.rs` | Ledger initialization — opens/creates database |
| `init_reuse.rs` | Database reuse logic for existing ledgers |
| `ops.rs` | Core operations (append op, read ops) |
| `ops_query.rs` | Operation query helpers |
| `ops_write_direct.rs` | Direct write operations |
| `ops_write_generated.rs` | Generated write operations |
| `node_ops.rs` | Node-level operations (CRUD for ledger nodes) |
| `node_check.rs` | Node consistency checking |
| `doc_lookup.rs` | Document lookup by UUID — fail-closed on miss |
| `listing.rs` | Document and repo listing |
| `inode_index.rs` | Inode-to-node mapping index |
| `metadata.rs` | Document metadata management |
| `snapshot.rs` | Content snapshot system |
| `source_control.rs` | Source control integration with ledger |
| `range.rs` | Op range queries |
| `merge.rs` | Merge entry point |
| `shadow_binding.rs` | Shadow branch binding management |
| `shadow_manager.rs` | Shadow repo lifecycle management |
| `shadow_transfer.rs` | Shadow data transfer operations |
| `traits.rs` | Repository trait definitions |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `manager/` | RepoManager — high-level repo operations orchestrator |
| `merge/` | Merge engine — diff, conflict resolution, CRDT merge |
| `node_meta/` | Node metadata management |
| `shadow/` | Shadow branch access and management |

## For AI Agents

### Working In This Directory

- **Ledger-first**: All mutations go through the ledger as facts before materializing to workspace.
- **UUID-first**: `doc_lookup.rs` must fail-closed on doc_id miss — never fall back to path-only.
- **Redb**: Embedded database — transactions are ACID but single-writer.
- **Shadow branches**: Remote peer state stored in separate Redb databases under `remotes/`.

<!-- MANUAL: -->

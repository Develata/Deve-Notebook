<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# docs

## Purpose
Document CRUD operations: create files/folders, rename files/directories, delete, and copy. All operations go through the ledger (Content Facts + Structure Facts) and must resolve to a local write scope. Remote branches are read-only and rejected. Path validation enforces security rules (no traversal, no absolute paths, depth limits, no `.notegit`, no `.md` directories).

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Entry points, path validation (`validate_file_path`/`validate_folder_path`), scope bootstrap |
| `create.rs` | Document creation orchestration (dispatches to file or folder) |
| `create_file.rs` | File creation: registers node in ledger, writes workspace file |
| `create_folder.rs` | Folder creation: registers structure fact |
| `rename.rs` | Rename/move orchestration |
| `rename_file.rs` | File rename with ledger mapping updates |
| `rename_dir.rs` | Directory rename with recursive structure fact updates |
| `delete.rs` | Document/folder deletion from ledger and workspace |
| `copy.rs` | Copy orchestration entry point |
| `copy_utils.rs` | Copy helpers for asset-only copying and tree walks |
| `copy_utils_test.rs` | Copy helper traversal and fail-closed regression tests |
| `errors/mod.rs` | Docs-specific error response helpers |
| `errors/classify.rs` | Docs-specific error classification |
| `file_register.rs` | File registration in ledger (apply_file_structure) |
| `node_helpers.rs` | Node tree manipulation and projection refresh helpers |
| `node_target.rs` | Target node resolution for operations |

## Subdirectories
| Directory | Purpose |
|-----------|---------|
| `copy/` | Directory and file copy implementation with ledger registration |

## For AI Agents

### Working In This Directory
- All write operations require local scope; `resolve_local_write_scope` rejects remote branches.
- Path validation (`mod.rs`) blocks: `..` traversal, absolute paths, depth > 10, `.notegit` reserved path, `.md` directory segments.
- `MAX_DEPTH = 10` is a hard limit on directory nesting.
- Operations must create appropriate ledger facts and broadcast `FsChangeDetected` after mutations.
- `checked_exists` and `checked_existing_is_dir` use fail-closed `try_exists`.

<!-- MANUAL: -->

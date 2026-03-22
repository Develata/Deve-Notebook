<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# manager

## Purpose

RepoManager — the high-level orchestrator for all repository operations. Manages repo catalog, database handles, projection, source control, snapshots, commits, structure operations, metadata, maintenance, and remote repo scanning. This is the primary API surface for the CLI server.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | RepoManager struct and module declarations |
| `core.rs` | Core repo operations — open, close, get_db |
| `core_dirs.rs` | Directory management within repos |
| `locator.rs` | Repo locator — finds repos by UUID or name |
| `repository.rs` | Repository trait implementation |
| `repo_db.rs` | Per-repo database handle management |
| `repo_catalog_entries.rs` | Repo catalog CRUD |
| `repo_info.rs` | Repo info and metadata queries |
| `repo_lookup.rs` | Repo lookup helpers |
| `types.rs` | Manager-specific types |
| `commit_apply.rs` | Commit application logic |
| `commit_ops.rs` | Commit operation helpers |
| `commit_plan.rs` | Commit planning |
| `commit_structure_plan.rs` | Structure-aware commit planning |
| `ops_ops.rs` | Op-level operations |
| `ops_structure.rs` | Structure operation helpers |
| `structure_ops.rs` | Structure mutation operations |
| `structure_projection.rs` | Structure projection computation |
| `structure_projection_support.rs` | Projection support helpers |
| `snapshot_ops.rs` | Snapshot operations |
| `metadata_ops.rs` | Metadata CRUD operations |
| `metadata_repair_ops.rs` | Metadata repair |
| `merge_ops.rs` | Merge orchestration |
| `maintenance.rs` | Database maintenance tasks |
| `projection_cleanup.rs` | Projection cleanup and orphan removal |
| `workspace.rs` | Workspace materialization |
| `source_control_api.rs` | Source control API surface |
| `source_control_ops.rs` | SC operation implementations |
| `source_control_query_ops.rs` | SC query operations |
| `source_control_target.rs` | SC target resolution |
| `source_control_target_lookup.rs` | SC target lookup |
| `source_control_path_target.rs` | SC path-based target (legacy) |
| `source_control_workdir.rs` | SC working directory operations |
| `source_control_workdir_db.rs` | SC workdir database operations |
| `source_control_workdir_helpers.rs` | SC workdir helpers |
| `dir_structure_plan.rs` | Directory structure planning |
| `dir_structure_support.rs` | Directory structure support |
| `local_repo_metadata_repair.rs` | Local repo metadata repair |
| `local_repo_names.rs` | Local repo name management |
| `local_repo_source_control_repair.rs` | Local repo SC repair |
| `remote_repo_allocate.rs` | Remote repo allocation |
| `remote_repo_scan.rs` | Remote repo scanning |
| `remote_repo_scan_entry.rs` | Remote repo scan entry processing |
| `remote_repo_scan_helpers.rs` | Remote repo scan helpers |

## For AI Agents

### Working In This Directory

- RepoManager is the main API for all repo operations — the CLI server calls into this.
- `source_control_target.rs` and `source_control_target_lookup.rs` must use UUID-first resolution.
- `projection_cleanup.rs` handles orphaned projection files.
- `locator.rs` is critical for repo discovery — fail-closed on missing catalog entries.

<!-- MANUAL: -->

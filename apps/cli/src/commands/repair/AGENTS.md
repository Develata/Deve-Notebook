<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# repair

## Purpose
Implements the `repair` CLI subcommand which recovers from various forms of local data corruption. Orchestrates multiple repair strategies in sequence: quarantining nil shadow repos, fixing repo-prefixed paths, quarantining invalid `.md` directories, restoring documents from backups, and optionally rebuilding the file projection.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Repair entry point; orchestrates all repair strategies in sequence |
| `shadow.rs` | Quarantines nil-UUID shadow repo databases into `.invalid/` directory |
| `path_fix.rs` | Repairs repo-prefixed paths (e.g., `reponame/notes/a.md` to `notes/a.md`) with atomic rollback |
| `weird_paths.rs` | Quarantines directories with `.md` extensions (invalid filesystem layout) with rollback |
| `restore.rs` | Restores corrupted documents from backup by diffing current vs backup content and patching |
| `rebuild.rs` | Rebuilds the file projection for repos via `SyncManager::rebuild_projection_local_repo` |
| `check_test/mod.rs` | Repair preflight check tests |
| `path_fix/tests.rs` | Path-fix repair tests |
| `path_fix/rollback_tests.rs` | Path-fix rollback tests |
| `restore/tests.rs` | Restore repair tests |
| `rebuild/tests.rs` | Projection rebuild tests |
| `weird_paths/tests.rs` | Weird-path quarantine tests |

## For AI Agents

### Working In This Directory
- All repair operations use fail-closed semantics: filesystem `try_exists` failures produce errors, not false negatives.
- `path_fix.rs` and `weird_paths.rs` both implement rollback: if ledger updates fail after filesystem moves, the filesystem change is reverted.
- `restore.rs` detects corruption by checking if workspace files start with `# Loading...` (a known corruption marker).
- `shadow.rs` explicitly rejects unexpected hidden directories and non-directory entries in the remotes folder to prevent silent data loss.
- Tests include Unix-specific permission tests (`#[cfg(unix)]`) that verify fail-closed behavior on unreadable paths.

<!-- MANUAL: -->

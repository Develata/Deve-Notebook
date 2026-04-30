# Git Import Resolved Roundtrip Smoke 2026-05-01

This report closes the `Git import resolved commit/export roundtrip smoke`
active queue item.

## Result

- Blocking failures: 0.
- An imported conflict resolved with `KeepFs` can be committed through the
  normal Source Control commit handler.
- The resolved staged entry does not retain pending/import conflict metadata
  after commit.
- The resulting Deve commit is queued in `git_mirror_commits` and can be
  exported back to Git through the explicit mirror/export path.
- After export, the mirror record is `Committed`, has a Git commit id, Git
  worktree status is clean, and `HEAD:note.md` matches the accepted imported
  filesystem content.

## Code Coverage Added

- `apps/cli/src/server/source_control_git_import_conflict_test.rs`
  - `resolved_import_keep_fs_commits_and_exports_to_git`

The new smoke fixture first exports the baseline Deve commit into Git so the
Git mirror mapping is real before import. It then creates ledger-only
divergence, applies a conflicting Git worktree change through `apply_import`,
resolves through the server `ResolveConflict` handler, commits through the
server Source Control commit handler, and finally runs `export_mirror`.

## Verified

```bash
cargo fmt --check
cargo test -p deve_cli resolved_import_keep_fs_commits_and_exports_to_git -- --nocapture
cargo test -p deve_cli source_control_git_import_conflict_test -- --nocapture
```

## Next Narrow Batch

Add a command-layer resolved-import/export roundtrip smoke. The next risk is
not the core mirror executor, but whether the public `deve_cli git import
--apply` / Source Control commit / `deve_cli git export` surface reports the
same state transitions and keeps user-facing guidance consistent.

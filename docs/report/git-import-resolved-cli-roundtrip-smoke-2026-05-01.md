# Git Import Resolved CLI Roundtrip Smoke 2026-05-01

This report closes the `Git import resolved CLI roundtrip smoke` active queue
item.

## Result

- Blocking failures: 0.
- The command-layer `git::import(..., apply=true)` path can import an external
  Git worktree modification into pending/import state.
- A conflict-producing imported pending entry can be resolved through normal
  Source Control staging with conflict metadata cleared before commit.
- The resulting Deve commit is queued for Git mirror export.
- The command-layer `git::export` path exports the resolved imported commit
  back to Git, marks the mirror record `Committed`, and leaves pending/staged
  Source Control state clean.
- Git `HEAD:note.md` matches the accepted imported filesystem content after
  export.

## Code Coverage Added

- `apps/cli/src/commands/git_import_smoke_test.rs`
  - `git_import_apply_resolved_commit_exports_roundtrip`

The smoke uses the public command helper layer for Git import/export and the
normal Source Control API for the required stage/commit step. There is no
separate current `deve_cli sc commit` command; the tested public CLI surface is
therefore `deve_cli git import --apply` plus `deve_cli git export`.

## Verified

```bash
cargo fmt --check
cargo test -p deve_cli git_import_apply_resolved_commit_exports_roundtrip -- --nocapture
cargo test -p deve_cli git_import -- --nocapture
cargo test -p deve_core git_bridge -- --nocapture
git diff --check
```

## Next Narrow Batch

Add an import/export/push smoke. The next risk is whether a resolved imported
commit that has been exported to Git can be published through the existing
`deve_cli git push` surface while preserving the same fail-closed blockers for
unexported or dirty mirror state.

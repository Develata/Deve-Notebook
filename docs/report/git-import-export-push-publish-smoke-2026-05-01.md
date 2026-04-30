# Git Import Export Push Publish Smoke 2026-05-01

This report closes the `Git import/export/push resolved publish smoke` active
queue item.

## Result

- Blocking failures: 0.
- A resolved imported change can move through the command-layer chain:
  `git import --apply` -> Source Control resolved stage/commit -> `git export`
  -> `git push`.
- Push before export is blocked and does not create the remote branch.
- Push after export publishes the mapped Git mirror `HEAD` to the configured
  bare remote/branch.
- Dirty Git worktree after export is blocked before the first successful push,
  and the remote branch remains absent until the dirty file is removed.
- The final remote branch points at the Git commit id stored in the
  `git_mirror_commits` record for the resolved imported Deve commit.

## Code Coverage Added

- `apps/cli/src/commands/git_import_smoke_test.rs`
  - `git_import_export_push_resolved_publish_roundtrip`

The smoke still treats `.notegit` / Deve ledger as authority. Git is only used
as the external mirror and remote publish surface.

## Verified

```bash
cargo fmt --check
cargo test -p deve_cli git_import_export_push_resolved_publish_roundtrip -- --nocapture
cargo test -p deve_cli git_import -- --nocapture
cargo test -p deve_core git_bridge -- --nocapture
```

## Next Narrow Batch

Run a Git mirror import/export/push docs/code drift rescan. The next risk is
stale plan/report wording now that the resolved-import chain has explicit
command-layer import, export, push, dirty-worktree blocker, and unexported
queue blocker coverage.

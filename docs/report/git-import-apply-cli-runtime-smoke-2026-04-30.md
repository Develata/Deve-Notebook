# Git Import Apply CLI Runtime Smoke 2026-04-30

This report closes the `Git import apply CLI/runtime smoke` active queue item.

## Result

- Blocking failures: 0.
- `deve_cli git import` dry-run remains read-only for pending/import state.
- `deve_cli git import --apply` writes safe modified/added Git worktree changes
  into Source Control pending/import state.
- Existing pending blockers fail closed and prevent partial writes for other
  candidates in the same import batch.
- CLI output copy continues to point operators back to Deve Source Control for
  stage/commit after import apply.

## Code Coverage Added

- `crates/core/src/git_bridge/import_apply.rs`
  - `plan_import_dry_run_does_not_write_pending_entries`
  - `apply_import_existing_pending_blocker_prevents_partial_writes`
- `apps/cli/src/commands/git_import_smoke_test.rs`
  - `git_import_command_dry_run_is_read_only_and_apply_writes_pending`
  - `git_import_command_apply_blocker_prevents_partial_pending_writes`

The CLI command tests initialize an isolated ledger/vault/Git repo, commit a
baseline through Deve and Git, then call `commands::git::import` through the
same command-layer function used by `deve_cli git import`.

## Verified

```bash
cargo fmt --check
cargo test -p deve_core git_bridge::import_apply
cargo test -p deve_cli git_import_command
cargo test -p deve_cli import_apply_report_lines_point_back_to_deve_source_control
```

## Next Narrow Batch

Run a Git-import conflict resolution runtime smoke. `--apply` can mark modified
tracked docs as conflicting via `has_conflict`; the next risk is whether Source
Control conflict resolution still exposes the imported pending entry correctly
and resolves through the ledger-authoritative stage/commit path.

# Git Import Conflict Resolution Runtime Smoke 2026-04-30

This report closes the `Git import conflict resolution runtime smoke` active
queue item.

## Result

- Blocking failures: 0.
- `deve_cli git import --apply` conflict entries keep `has_conflict = true`
  while they are still pending.
- Source Control conflict resolution now clears the conflict marker when
  `KeepFs` moves an imported pending entry into staged state.
- `KeepLedger` discards the imported pending entry, restores the workspace from
  the current Ledger projection, and does not create staged changes.
- Both resolution paths emit scoped `ConflictResolved` messages with `repo_id`
  and browser `scope_nonce`.

## Code Coverage Added

- `apps/cli/src/server/source_control_git_import_conflict_test.rs`
  - `imported_conflict_keep_fs_resolves_to_clean_staged_entry`
  - `imported_conflict_keep_ledger_discards_import_without_staging`
- `crates/core/src/ledger/manager/source_control_target.rs`
  - Added `stage_resolved_pending_target_in_local_repo` so conflict resolution
    can clear pending-only conflict metadata before staging.

The tests initialize an isolated Deve repo and Git repo, commit a shared
baseline, create ledger-only divergence, apply a conflicting Git worktree
change through `apply_import`, and then call the same server conflict handler
used by WebSocket Source Control messages.

## Verified

```bash
cargo fmt --check
cargo test -p deve_cli imported_conflict
cargo test -p deve_cli git_import_command
cargo test -p deve_core git_bridge
```

## Next Narrow Batch

Run a resolved-import roundtrip smoke. The next risk is not conflict detection
itself, but whether the resolved `KeepFs` staged entry can be committed through
normal Deve Source Control and then exported back to Git without bypassing
`.notegit` authority or leaving stale import/conflict metadata.

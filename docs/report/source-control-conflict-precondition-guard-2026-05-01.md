# Source Control Conflict Precondition Guard 2026-05-01

This report closes the review follow-up after
`git-import-conflict-resolution-runtime-smoke-2026-04-30.md`.

## Result

- Blocking failures: 0.
- `ResolveConflict` is now guarded server-side: the resolved pending entry must
  have `has_conflict = true`.
- Non-conflict pending entries are rejected with structured
  `SC_CONFLICT_TARGET_MISSING` and a scoped protocol error.
- Rejected non-conflict resolve requests do not move pending entries into
  staging and do not discard workspace content.

## Code Coverage Added

- `apps/cli/src/server/source_control_git_import_conflict_test.rs`
  - `resolve_conflict_rejects_non_conflict_pending_entry`
- `apps/cli/src/server/handlers/source_control/service/target.rs`
  - Added `resolved_target_entry` so handlers can enforce preconditions on the
    exact resolved change entry instead of trusting the client-selected path.

## Verified

```bash
cargo fmt --check
cargo test -p deve_cli resolve_conflict_rejects_non_conflict_pending_entry
cargo test -p deve_cli source_control_git_import_conflict_test
```

## Next Narrow Batch

Continue with the active queue item:
`Git import resolved commit/export roundtrip smoke`.

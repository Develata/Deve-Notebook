# Post-Verification Plan/Code Drift Rescan 2026-04-30

This report closes the active rescan after the full workspace verification pass
and Docker cargo-chef cleanup.

## Scope

- Re-read current active queue, plan/features/acceptance docs, and code-level
  stale/future markers.
- Exclude explicitly future-only work: server-backed Settings API, graph
  renderer dependencies, executable Web Git repair UI, native Tauri packaging,
  background Git writer, and MCP runtime.
- Prefer narrow plan/code mismatches that affect operator-facing behavior.

## Finding Closed In This Batch

`deve_cli git import` dry-run output still described apply as a future path even
though `deve_cli git import --apply` is now implemented and documented as the
current explicit pending/import write surface.

Fixed:

- `apps/cli/src/commands/git_output.rs` now tells operators to rerun with
  `--apply` to write pending/import, not ledger.
- `apps/cli/src/commands/git_output_test.rs` now asserts the current wording.
- `docs/plan/12_commands.md` now describes dry-run output as changes that can
  enter pending/import through `--apply`, not a future path.

## Verified

```bash
cargo test -p deve_cli import_plan_lines_are_explicitly_dry_run_and_non_authoritative
rg -n "future apply|future import apply|将来可进入 pending/import" apps/cli docs/plan docs/features docs/acceptance-cases -g '*.rs' -g '*.md'
```

The first test passed with 1 matching test. The stale-copy scan returned no
matches in code or authoritative plan/features/acceptance docs.

## Next Narrow Batch

Run a real CLI/runtime-level smoke for `deve_cli git import --apply` against an
isolated repo. The unit-level output test is now aligned, but the current
operator contract should also be verified end-to-end:

- dry-run reports Git worktree changes without writing ledger/pending state;
- `--apply` writes safe changes into pending/import;
- blockers fail closed without partial pending/import writes;
- output continues to point back to Deve Source Control for stage/commit.

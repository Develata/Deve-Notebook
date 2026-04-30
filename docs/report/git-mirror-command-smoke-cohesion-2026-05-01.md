# Git Mirror Command Smoke Cohesion 2026-05-01

This report closes the `Git mirror command smoke cohesion review` active queue
item.

## Result

- Blocking failures: 0.
- `apps/cli/src/commands/git_import_smoke_test.rs` was split so scenario tests
  no longer carry the Git/ledger fixture machinery inline.
- `apps/cli/src/commands/git_import_smoke_support.rs` now owns shared Git
  command helpers, exported-baseline setup, resolved-import setup, and push
  report assertions.
- The command-layer push smoke now asserts the structured blocker locations and
  reason fragments for both unexported queue and dirty Git worktree cases,
  instead of only checking that the remote branch remains absent.
- `docs/acceptance-cases/04_diff.md` now marks the Git mirror import/push gates
  as CLI assertions, not UI assertions.

## Cohesion Status

- `git_import_smoke_test.rs`: 198 lines.
- `git_import_smoke_support.rs`: 240 lines.
- Both files are below the current soft 250-line cohesion warning threshold.
- Further Git mirror smoke growth should add a new scenario file rather than
  expanding either file past the soft threshold.

## Verified

```bash
cargo fmt --check
wc -l apps/cli/src/commands/git_import_smoke_test.rs apps/cli/src/commands/git_import_smoke_support.rs
cargo test -p deve_cli git_import_export_push_resolved_publish_roundtrip -- --nocapture
cargo test -p deve_cli git_import_apply_resolved_commit_exports_roundtrip -- --nocapture
cargo test -p deve_core push_mirror_refuses_unexported_queue_without_touching_remote -- --nocapture
```

## Next Narrow Batch

Run a post-Git-mirror priority reselection pass before opening the next
implementation domain. The Git mirror import/export/push path is now at a clean
handoff point for the current plan: CLI surface, resolved import chain, export,
push blockers, docs-code drift, and smoke cohesion have all been covered.

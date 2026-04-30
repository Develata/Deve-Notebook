# Git Mirror Import Export Push Drift Rescan 2026-05-01

This report closes the `Git mirror import/export/push docs-code drift rescan`
active queue item.

## Result

- Blocking documentation drift found: 1.
- `docs/plan/04_storage.md` still said Git import must generate Deve ledger
  facts. That was stale. Current code and tests implement `git import --apply`
  as pending/import only; ledger facts are generated only by subsequent Deve
  Source Control stage/commit.
- Plan wording now explicitly describes the resolved import chain:
  `git import --apply` -> Source Control resolved stage/commit -> `git export`
  -> `git push`.
- Acceptance wording now includes unexported queue blocker and dirty Git
  worktree blocker as publish gates.

## Code Evidence

- `apps/cli/src/commands/git_import_smoke_test.rs`
  - `git_import_apply_resolved_commit_exports_roundtrip`
  - `git_import_export_push_resolved_publish_roundtrip`
- `apps/cli/src/server/source_control_git_import_roundtrip_test.rs`
  - `resolved_import_keep_fs_commits_and_exports_to_git`
- `crates/core/src/git_bridge/push_test.rs`
  - `push_mirror_refuses_unexported_queue_without_touching_remote`
  - `push_mirror_pushes_exported_head_to_remote`

## Docs Updated

- `docs/plan/04_storage.md`
- `docs/plan/07_diff_logic.md`
- `docs/plan/12_commands.md`
- `docs/plan/14_tech_stack.md`
- `docs/plan/验收清单.md`
- `docs/features/12_commands.md`
- `docs/acceptance-cases/04_diff.md`

## Verified

```bash
! rg "Git import 必须生成 Deve ledger facts" docs/plan docs/features docs/acceptance-cases
rg "Git import 只能进入 pending/import|git import --apply.*pending/import" docs/plan docs/features docs/acceptance-cases
rg "git_import_export_push_resolved_publish_roundtrip|push_mirror_refuses_unexported" apps/cli/src crates/core/src
```

## Next Narrow Batch

Run a narrow review/verification pass over the Git mirror command-layer smoke
files before selecting the next implementation domain. The main remaining
engineering risk is test-file cohesion: `git_import_smoke_test.rs` is still
below the hard fuse, but further Git smoke growth should move shared helpers
into a dedicated support module.

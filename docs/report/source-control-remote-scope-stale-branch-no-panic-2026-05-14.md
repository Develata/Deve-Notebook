# Source Control Remote Scope Stale Branch No-Panic - 2026-05-14

## Scope

- Runtime surface: repo scope recovery, Source Control remote scope resolution, branch switcher current-scope mapping.
- Plan basis: `docs/plan/06_repository.md#repo-scope-runtime` and `docs/plan/07_diff_logic.md#source-control-runtime`.

## Change

- Replaced stale remote-scope `active_branch.expect("checked active branch")` detail construction with explicit `if let Some(branch)` bindings.
- Kept existing `ScStaleScope` / `ScRepoContextInvalid` mapping semantics unchanged.
- Added Source Control baseline guards so these stale-scope paths cannot regain branch `expect` calls.

## Verification

- `cargo test -p deve_cli source_control_scope_binding -- --nocapture`
- `cargo test -p deve_cli sync_scope_cleanup -- --nocapture`
- `cargo test -p deve_cli switcher_current_scope -- --nocapture`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`

## Result

Remote stale-scope failures remain structured errors and no longer depend on panic-backed branch invariants for diagnostic detail.

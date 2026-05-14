# Git Status Retry Hint No-Panic - 2026-05-14

## Scope

- Runtime surface: `deve_cli git status` Git mirror lagging-record output.
- Plan basis: `docs/plan/07_diff_logic.md#git-mirror-lifecycle` and `docs/plan/12_commands.md#cli-commands`.

## Change

- Replaced the lagging-record retry command `expect` with an explicit retry command value.
- Kept status output ordering, repair guidance, and retry command text unchanged.
- Added Source Control baseline guards so the status renderer cannot regain the panic-backed retry hint invariant.

## Verification

- `cargo test -p deve_cli status_lines_include_per_commit_lag_and_retry_hint -- --nocapture`
- `cargo test -p deve_cli status_lines_include_cli_only_repair_action -- --nocapture`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`

## Result

Git mirror status remains read-only and now renders lagging retry guidance without a panic-backed display-layer invariant.

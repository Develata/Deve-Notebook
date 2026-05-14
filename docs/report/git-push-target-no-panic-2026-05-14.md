# Git Push Target No-Panic

Date: 2026-05-14

## Scope

- `crates/core/src/git_bridge/push.rs`
- `crates/core/src/git_bridge/push/tests.rs`
- `scripts/check-source-control-baseline.sh`

## Contract

- `docs/plan/07_diff_logic.md#git-mirror-lifecycle`
- `docs/plan/12_commands.md#cli-commands`

## Change

- Replaced `expect("remote resolved")` and `expect("branch resolved")` before `git push` with an explicit final target guard.
- Missing remote or branch now produces `git_remote` blockers and returns a non-pushed `GitMirrorPushReport`.
- Existing preflight, mapping, remote URL lookup, and successful `git push` behavior remain unchanged.
- Added a unit test and Source Control baseline guards to keep unresolved push targets fail-closed instead of panic-backed.

## Verification

- `cargo test -p deve_core unresolved_push_target -- --nocapture`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`

# Command Palette Establish Branch Boundary - 2026-05-14

## Scope

- `P2P: Establish Branch` had no backend contract but previously opened peer branch search.
- This batch keeps the command discoverable and marks it unavailable.
- The command now emits a local Source Control notice only; it does not switch branch, create branch, or call a backend writer.

## Code Changes

- Added command availability metadata for Command Palette entries.
- Rendered unavailable commands with `aria-disabled` and `data-deve-command-unavailable`.
- Surfaced unavailable command reasons in unified command search results.
- Added `establish-branch-unavailable` Source Control notice copy.

## Verification

- `cargo test -p deve_web establish_branch_command -- --nocapture`
- `cargo test -p deve_web command_provider -- --nocapture`
- `cargo test -p deve_web source_control_notice -- --nocapture`
- `cargo test -p deve_web establish_branch_notice -- --nocapture`
- `cargo test -p deve_web command_palette -- --nocapture`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/check-cli-settings-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`
- `cargo clippy -p deve_web --all-targets -- -D warnings`
- `cargo fmt --check`
- `git diff --check`

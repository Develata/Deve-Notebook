# Settings Sync UI Acceptance Depth - 2026-04-30

## Scope

Closed the next P2 Settings UI acceptance-depth gap for `SET-005`: sync mode
visual feedback is now driven by a tested policy helper instead of a one-shot
boolean captured during view construction.

## Implemented

- Moved Settings section button state decisions into
  `settings_sections_policy.rs`.
- `SyncModeSection` now derives button classes from the current `sync_mode`
  signal, so Auto / Manual visual selection tracks state changes explicitly.
- Added policy tests for auto mode, manual mode, and unknown-mode safe default.
- Kept callbacks unchanged: Auto still sends `auto`, Manual still sends
  `manual`.

## Boundary

This is UI feedback only. It does not change sync protocol behavior, merge
policy, ledger authority, or persisted runtime config.

## Verification

- `cargo test -p deve_web settings -- --nocapture`
- `cargo test -p deve_web ai_backend -- --nocapture`
- `scripts/check-cli-settings-baseline.sh`
- `scripts/check-ai-baseline.sh`
- `cargo fmt --check`

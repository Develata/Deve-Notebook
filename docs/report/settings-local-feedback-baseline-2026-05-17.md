# Settings Local Feedback Baseline - 2026-05-17

本报告记录 Settings Local Persistence / Feedback Closure 的 targeted baseline。`docs/plan/` 未修改。

## Scope

- Covered acceptance:
  - `SET-003`: requested `trusted-cli` is preserved while effective backend falls back to `native`.
  - `SET-004`: file-backed `config.toml` values apply after reload/restart.
  - `SET-005`: Settings UI changes expose immediate local feedback.
  - `SET-006`: unavailable or reserved Settings controls expose visible and accessible reasons.
  - `SET-007`: server-backed Settings API remains future-only.
- Non-goal:
  - Server-backed Settings API.
  - Separate `settings.toml`.
  - GUI persistence for runtime config.
  - Web authority writes outside current UI preference state.

## Changes

- Added `scripts/check-settings-local-feedback-baseline.sh`.
- Bound the new script to `SET-001..007` where Settings local persistence or feedback is asserted.
- Corrected stale `config print` quote expectation in `SET-001`.
- Kept `check-cli-settings-baseline.sh` as the broad command/settings guard and used the new script as the focused Settings batch guard.

## Verification

Ran:

- `shellcheck scripts/check-settings-local-feedback-baseline.sh scripts/check-cli-settings-baseline.sh`
- `bash -n scripts/check-settings-local-feedback-baseline.sh scripts/check-cli-settings-baseline.sh`
- `bash scripts/check-settings-local-feedback-baseline.sh`

The focused script runs:

- `cargo test -p deve_cli config -- --nocapture`
- `cargo test -p deve_cli trusted_cli_untrusted -- --nocapture`
- `cargo test -p deve_cli backend_capabilities_http -- --nocapture`
- `cargo test -p deve_web settings -- --nocapture`
- `cargo test -p deve_web ai_backend -- --nocapture`
- `cargo test -p deve_web backend_for_send -- --nocapture`

Result: all checks passed.

## Decision

Settings local config and feedback have a repeatable baseline.

Next step: run a browser smoke for Settings open -> language feedback -> sync mode feedback -> AI backend disabled/available feedback -> reserved Hybrid marker -> reload boundary.

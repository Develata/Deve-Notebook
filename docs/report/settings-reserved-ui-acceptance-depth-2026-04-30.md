# Settings Reserved UI Acceptance Depth - 2026-04-30

## Scope

Closed the `SET-006` Settings UI acceptance-depth gap for reserved/future
controls: the Hybrid Editing placeholder now exposes a tested disabled state
and an explicit current-release-unavailable reason.

## Implemented

- Added `reserved_setting_state(Locale)` to the Settings section policy module.
- Hybrid Editing placeholder now derives class, disabled marker, title, and
  visible reason from that policy.
- Replaced the stale `Phase 6` copy with current-boundary wording:
  `Future setting: not available in the current release`.
- Added policy and i18n tests for the reserved setting disabled reason.

## Boundary

This remains a future/reserved UI placeholder. It does not enable hybrid
editing persistence, a Settings API, or authority mutation from Settings.

## Verification

- `cargo test -p deve_web settings -- --nocapture`
- `scripts/check-cli-settings-baseline.sh`
- `cargo fmt --check`

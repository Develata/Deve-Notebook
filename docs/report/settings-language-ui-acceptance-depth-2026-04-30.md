# Settings Language UI Acceptance Depth - 2026-04-30

## Scope

Closed the next P2 Settings UI acceptance-depth gap for `SET-005`: language
button visual feedback is now driven by a tested policy helper instead of
inline component class branching.

## Implemented

- Added `language_button_state(Locale)` to the Settings section policy module.
- Settings modal now derives English / Chinese button classes from this policy.
- Existing click behavior is unchanged: selecting English or Chinese still
  persists the locale preference and updates the locale signal.
- Added policy tests for English-active and Chinese-active visual states.

## Boundary

This is UI feedback only. It does not change locale storage authority, i18n
resource loading, command palette language switching, or runtime config.

## Verification

- `cargo test -p deve_web settings -- --nocapture`
- `scripts/check-cli-settings-baseline.sh`
- `cargo fmt --check`

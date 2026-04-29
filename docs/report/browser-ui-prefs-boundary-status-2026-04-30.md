# Browser UI Prefs Boundary Status - 2026-04-30

## Scope

Closed the P2 task from `docs/report/next-tasks.md`: consolidate harmless browser UI preferences behind the non-authoritative fallback storage layer.

## Current Boundary

- `apps/web/src/storage/prefs.rs` is the only functional localStorage entry point for UI prefs.
- `apps/web/src/storage/js_bridge.rs` may still probe `localStorage` capabilities.
- Layout widths, Outline visibility, locale preference, and shortcut overrides now use the prefs fallback layer.
- Repo identity, sync vector, writer readiness, auth secrets, scope nonce, and business facts remain forbidden in UI prefs.

## Verification

- `scripts/check-browser-prefs-boundary.sh`
- `cargo test -p deve_web typed_prefs_roundtrip -- --nocapture`
- `cargo test -p deve_web shortcut_config_roundtrips -- --nocapture`
- `cargo test -p deve_web locale_preference_uses_ui_prefs -- --nocapture`
- `cargo fmt --check`
- `scripts/plan-coverage.sh`
- `git diff --check`

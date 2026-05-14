# Ctrl Key DOM Helper No-Panic - 2026-05-14

## Scope

- Runtime surface: Web Ctrl/Cmd link activation modifier hook.
- Plan basis: `docs/plan/13_settings.md#keyboard-shortcuts` and `docs/features/operations/rendering_link_activation_gate.md`.

## Change

- Replaced `window` / `document` / `body` panic-backed access in `use_ctrl_key`.
- Added explicit `Option` helpers for `window`, `document`, and `body`.
- Kept keydown add-class, keyup remove-class, and blur cleanup behavior unchanged when DOM is available.
- Added a native unit test for no-window fallback.
- Added rendering baseline guards so the link activation hook cannot regain DOM `expect` / `unwrap` paths.

## Verification

- `cargo test -p deve_web ctrl_key -- --nocapture`
- `bash scripts/check-rendering-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`

## Result

Ctrl/Cmd link activation remains CSS-class driven in normal browser runtime and now fails soft when DOM globals are unavailable.

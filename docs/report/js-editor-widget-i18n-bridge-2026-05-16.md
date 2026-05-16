# JS Editor Widget I18N Bridge

Date: 2026-05-16

## Scope

- Close the P1 gap from `mainline-gap-scan-after-full-regression-2026-05-16.md`.
- Keep Rust `t::*` as the authoritative i18n source.
- Localize only visible CodeMirror widget copy in `apps/web/js/extensions`.
- Do not change `docs/plan/`.

## Changes

- Added `apps/web/src/i18n/js_bridge.rs` to publish editor widget copy to `window.deve_i18n`.
- Added `apps/web/js/i18n.js` as the JS-side read-only bridge for editor widgets.
- Replaced Code Toolbar, Code Menu, and Mermaid visible hardcoded copy with bridge lookups.
- Extended `scripts/check-i18n-hardcoded-baseline.sh` to cover `apps/web/js/extensions`.
- Updated `docs/features/operations/i18n_hardcoded_audit.md` so the operation path includes the JS widget surface.

## Verification

- `bash scripts/check-i18n-hardcoded-baseline.sh`
- `cargo test -p deve_web editor_widget_copy -- --nocapture`
- `npm --prefix apps/web run build`
- `cargo check -p deve_web --target wasm32-unknown-unknown`
- `cargo fmt --check`
- `bash scripts/check-acceptance-bindings.sh`

## Result

The editor widget copy now follows the same locale state as the Rust UI. JS fallback copy remains local to the bridge for fail-soft behavior, while widget modules no longer own visible English strings.

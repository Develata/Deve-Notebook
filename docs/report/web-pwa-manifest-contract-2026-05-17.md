# Web PWA Manifest Contract

Date: 2026-05-17

## Scope

- Plan chapter: `docs/plan/08_ui_design_01_web.md` §5 PWA Support.
- Contract closed: Web PWA manifest surface for standalone install metadata.
- No `docs/plan/` change.
- No service worker, offline authority, native runtime, or additional browser storage behavior was introduced.

## Changes

- Added `apps/web/public/manifest.json`.
- Added `<link rel="manifest" href="/manifest.json" />` to `apps/web/index.html`.
- Added `<meta name="theme-color" content="#1e1e1e" />` to `apps/web/index.html`.
- Added Trunk copy binding for `public/manifest.json`.
- Added `UI-WEB-006` and bound it to `scripts/check-ui-desktop-baseline.sh`.

## Verification

- `bash scripts/check-ui-desktop-baseline.sh`
- `node -e ... apps/web/public/manifest.json`
- `cd apps/web && NO_COLOR=true BROWSERSLIST_IGNORE_OLD_DATA=true trunk build`
- `node -e ... apps/web/dist/manifest.json`
- `rg -n "manifest.json|theme-color|standalone|#1e1e1e" apps/web/dist/index.html apps/web/dist/manifest.json`

## Result

- Browser install metadata now exposes `display=standalone` and `theme_color=#1e1e1e`.
- The manifest is copied into `apps/web/dist` by the normal Trunk build.
- Web offline behavior remains limited to the existing PWA/static asset boundary.
- Acceptance counters: automated `149`, feature walkthrough `54`, manual `0`, unbound `0`.

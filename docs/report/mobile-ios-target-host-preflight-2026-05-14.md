# Mobile iOS Target-host Preflight

Date: 2026-05-14

## Scope

Ran the Mobile package preflight in iOS-only mode on the current Linux/WSL host.

No iOS project generation or package build was opened.

## Result

- Default diagnostic mode passed without claiming iOS readiness.
- Required mode failed closed on this non-macOS host.
- Missing prerequisites reported:
  - `cargo tauri ios subcommand`
  - `iOS target-host requires macOS`
- Mobile native-packaging shell/package acceptance tests still pass before target-host diagnostics.

## Boundary

- `cargo tauri ios init` remains closed.
- `cargo tauri ios build` remains closed.
- `apps/mobile/gen/apple` remains forbidden.
- Native child-process runtime remains closed.
- Mobile shell does not gain ledger/vault/source-control/search/`.git`/`.notegit` authority.

## Verification

- `DEVE_MOBILE_PACKAGE_TARGETS=ios scripts/check-mobile-platform-package-preflight.sh`
- `DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1 DEVE_MOBILE_PACKAGE_TARGETS=ios scripts/check-mobile-platform-package-preflight.sh`

The required command is expected to exit non-zero on Linux/WSL.

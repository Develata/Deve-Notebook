# Mobile Platform Package Build Preflight

Date: 2026-05-14

## Scope

- Added `scripts/check-mobile-platform-package-preflight.sh`.
- Added release/runbook/acceptance coverage for the Mobile package preflight gate.
- Kept Mobile package build diagnostic-only unless explicitly required by environment.

## Boundary

- Android/iOS project generation remains closed.
- `cargo tauri android build` and `cargo tauri ios build` remain closed.
- Mobile runtime entrypoint and build script remain forbidden in this gate.
- Native child-process runtime and authority writes remain closed.
- Linux/WSL runs may diagnose Android prerequisites; iOS readiness requires macOS.

## Required Mode

`DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1` fails closed when selected target
prerequisites are missing.

`DEVE_MOBILE_PACKAGE_TARGETS=android` or `DEVE_MOBILE_PACKAGE_TARGETS=ios`
narrows prerequisite diagnostics.

## Verification

- `scripts/check-mobile-platform-package-preflight.sh`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-packaging-gate.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-dev-runbook-baseline.sh`

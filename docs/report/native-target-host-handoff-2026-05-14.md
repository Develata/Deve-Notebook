# Native Target-host Handoff

Date: 2026-05-14

## Scope

This report freezes the next target-host package execution steps after local
Desktop/Mobile packaging gates passed on Linux/WSL.

No `docs/plan/` contract changed in this batch.

## Desktop macOS

Run on a macOS target host:

```bash
DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1 DEVE_DESKTOP_TARGET_HOSTS=macos scripts/check-desktop-target-host-preflight.sh
DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 scripts/check-desktop-platform-package-build.sh
```

Required evidence:

- Host OS and tool versions.
- Preflight command output.
- Package build command output.
- Artifact paths.
- Install result.
- Startup smoke result.
- Confirmation that child-process runtime and native authority writes remain closed.

## Desktop Windows

Run on a Windows target host:

```bash
DEVE_DESKTOP_TARGET_HOST_PREFLIGHT_REQUIRED=1 DEVE_DESKTOP_TARGET_HOSTS=windows scripts/check-desktop-target-host-preflight.sh
DEVE_DESKTOP_PACKAGE_BUILD_REQUIRED=1 DEVE_DESKTOP_PACKAGE_BUNDLES=msi,nsis scripts/check-desktop-platform-package-build.sh
```

Required evidence matches Desktop macOS.

## Mobile iOS

Run on a macOS target host:

```bash
DEVE_MOBILE_PACKAGE_PREFLIGHT_REQUIRED=1 DEVE_MOBILE_PACKAGE_TARGETS=ios scripts/check-mobile-platform-package-preflight.sh
```

The current iOS gate is preflight-only. Do not run `cargo tauri ios init` or
`cargo tauri ios build` until an explicit iOS shell-only package execution gate
is added.

Required evidence:

- Host OS and tool versions.
- Required preflight command output.
- Missing prerequisites or readiness result.
- Confirmation that `apps/mobile/gen/apple` remains absent unless a later
  iOS package execution gate explicitly permits it.
- Confirmation that child-process runtime and native authority writes remain closed.

## Boundary

- Android shell APK execution is already verified separately.
- Desktop macOS/Windows package execution remains target-host work.
- Mobile iOS package execution remains target-host work.
- Process runtime gate remains closed until Desktop target-host and Mobile iOS
  evidence exists or the plan is explicitly reopened.

## Verification

- `scripts/check-release-baseline.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`

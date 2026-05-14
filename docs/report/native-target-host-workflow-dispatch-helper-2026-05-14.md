# Native Target-host Workflow Dispatch Helper

Date: 2026-05-14

## Scope

Added a local helper for the manual Native Target Host workflow:

- `scripts/dispatch-native-target-host-workflow.sh`

This batch does not modify `docs/plan/`, does not run target-host packages, and
does not open process runtime or native authority writes.

## Behavior

- The helper validates workflow input values locally.
- Default mode is dry-run and only prints the exact `gh workflow run` command.
- Remote dispatch requires `DEVE_NATIVE_TARGET_HOST_DISPATCH=1` and an
  authenticated GitHub CLI.
- The helper supports `all`, `desktop-macos`, `desktop-windows`, and
  `mobile-ios`.

## Verification

- `bash -n scripts/dispatch-native-target-host-workflow.sh`
- `scripts/dispatch-native-target-host-workflow.sh`
- `DEVE_NATIVE_TARGET_HOST_TARGET=desktop-windows DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT=true DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true scripts/dispatch-native-target-host-workflow.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `git diff --check`

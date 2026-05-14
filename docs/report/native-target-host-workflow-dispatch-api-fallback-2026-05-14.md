# Native Target-host Workflow Dispatch API Fallback

Date: 2026-05-14

## Scope

Strengthened the manual Native Target Host workflow dispatch helper:

- `scripts/dispatch-native-target-host-workflow.sh`

This batch does not modify `docs/plan/`, does not execute target-host packages,
and does not open process runtime or native authority writes.

## Behavior

- Default mode remains dry-run.
- The helper still prefers authenticated GitHub CLI dispatch.
- If `gh` is unavailable or unauthenticated, the helper can dispatch through the
  GitHub REST API using `DEVE_GITHUB_TOKEN`, `GH_TOKEN`, or `GITHUB_TOKEN`.
- Repository resolution uses `DEVE_NATIVE_TARGET_HOST_REPOSITORY`,
  `GITHUB_REPOSITORY`, or the `origin` remote.

## Local Result

- GitHub MCP dispatch was attempted twice and failed at TLS handshake timeout.
- Local environment has no `gh`, `GH_TOKEN`, or `GITHUB_TOKEN`.
- The helper now prints both the CLI command and API endpoint in dry-run mode.

## Verification

- `bash -n scripts/dispatch-native-target-host-workflow.sh`
- `scripts/dispatch-native-target-host-workflow.sh`
- `DEVE_NATIVE_TARGET_HOST_TARGET=desktop-windows DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT=true DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true scripts/dispatch-native-target-host-workflow.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `git diff --check`

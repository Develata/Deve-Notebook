# Native Target-host Latest Run Resolution

Date: 2026-05-14

## Scope

Extended the Native Target Host evidence collector:

- `scripts/collect-native-target-host-evidence.sh`

This batch does not modify `docs/plan/`, does not execute target-host packages,
and does not open process runtime or native authority writes.

## Behavior

- `DEVE_NATIVE_TARGET_HOST_RUN_ID=latest` resolves the most recent
  `native-target-host.yml` `workflow_dispatch` run for the selected ref.
- Latest run lookup prefers authenticated GitHub CLI.
- If `gh` is unavailable or unauthenticated, lookup uses the GitHub REST API
  with `DEVE_GITHUB_TOKEN`, `GH_TOKEN`, or `GITHUB_TOKEN`.
- After resolving the run id, the existing collector path downloads evidence
  artifacts and validates each report.

## Verification

- `bash -n scripts/collect-native-target-host-evidence.sh`
- `scripts/collect-native-target-host-evidence.sh`
- `DEVE_NATIVE_TARGET_HOST_RUN_ID=latest scripts/collect-native-target-host-evidence.sh`
- `DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 DEVE_GH_BIN=definitely-no-gh DEVE_NATIVE_TARGET_HOST_RUN_ID=latest scripts/collect-native-target-host-evidence.sh || true`
- `scripts/check-release-baseline.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `git diff --check`

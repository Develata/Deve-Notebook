# Native Target-host Evidence Collector

Date: 2026-05-14

## Scope

Added a local collector for Native Target Host workflow evidence artifacts:

- `scripts/collect-native-target-host-evidence.sh`

This batch does not modify `docs/plan/`, does not execute target-host packages,
and does not open process runtime or native authority writes.

## Behavior

- Default mode is dry-run.
- Real collection requires `DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1` and
  `DEVE_NATIVE_TARGET_HOST_RUN_ID=<run-id>`.
- The collector prefers authenticated GitHub CLI artifact download.
- If `gh` is unavailable or unauthenticated, it can use the GitHub REST API with
  `DEVE_GITHUB_TOKEN`, `GH_TOKEN`, or `GITHUB_TOKEN`.
- Every downloaded evidence Markdown file is validated by
  `scripts/check-native-target-host-evidence.sh`.

## Verification

- `bash -n scripts/collect-native-target-host-evidence.sh`
- `scripts/collect-native-target-host-evidence.sh`
- `DEVE_NATIVE_TARGET_HOST_RUN_ID=123456 scripts/collect-native-target-host-evidence.sh`
- `DEVE_NATIVE_TARGET_HOST_EVIDENCE_COLLECT=1 DEVE_GH_BIN=definitely-no-gh DEVE_NATIVE_TARGET_HOST_RUN_ID=123456 scripts/collect-native-target-host-evidence.sh || true`
- `scripts/check-release-baseline.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `git diff --check`

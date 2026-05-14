# Native Target-host Evidence Validator

Date: 2026-05-14

## Scope

Added a small evidence template and validator for target-host package execution:

- `docs/report/native-target-host-evidence-template.md`
- `scripts/check-native-target-host-evidence.sh`

This batch does not modify `docs/plan/`, does not open iOS package execution,
does not open child-process runtime, and does not grant native authority writes.

## Contract

Target-host evidence must explicitly record:

- target and workflow/local run reference
- host OS and tool versions
- exact commands
- package artifact paths or explicit N/A reason
- install result
- startup result
- `Process runtime gate: closed`
- `Native authority writes: closed`
- final conclusion

## Verification

- `scripts/check-native-target-host-evidence.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `scripts/check-release-baseline.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `git diff --check`

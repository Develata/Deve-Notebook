# Native Target-host Evidence Artifacts

Date: 2026-05-14

## Scope

Added workflow-generated target-host evidence artifacts.

Changed:

- `scripts/write-native-target-host-evidence.sh`
- `.github/workflows/native-target-host.yml`
- `docs/dev-runbook.md`
- `scripts/check-release-baseline.sh`
- `scripts/AGENTS.md`

This batch does not modify `docs/plan/`, does not run target-host packages
locally, does not open iOS package execution, and does not open process runtime
or native authority writes.

## Behavior

- Each target-host workflow job writes a validated evidence Markdown file under
  `target/native-target-host-evidence/`.
- Each target-host workflow job uploads that evidence as a separate artifact.
- Desktop package artifacts remain separate from evidence artifacts.
- iOS evidence explicitly records that iOS package execution remains closed.

## Verification

- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/native-target-host.yml")'`
- `bash -n scripts/write-native-target-host-evidence.sh`
- `scripts/write-native-target-host-evidence.sh`
- `scripts/check-native-target-host-evidence.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `scripts/check-release-baseline.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `git diff --check`

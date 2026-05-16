# Release Platform Acceptance Boundary Alignment - 2026-05-16

本报告记录 `REL-005` release/platform 验收边界对齐。`docs/plan/` 未修改。

## Scope

- Acceptance scope: `docs/acceptance-cases/12_tech_release.md`.
- Guard scope: `scripts/check-release-baseline.sh`.
- Source basis: current release plan, platform artifact consumption runbook, and target-host evidence reports.

## Result

- `REL-005` now states the combined release boundary: embedded frontend single binary plus shell-only target-host platform evidence.
- `REL-005` now explicitly excludes signed release readiness, store distribution readiness, physical-device readiness, native process runtime, and native authority writes.
- Release baseline now guards the exclusion assertions.

## Boundary

- This does not open Desktop or Mobile native process runtime.
- This does not add native authority writes.
- This does not claim signed macOS/Windows release readiness.
- This does not claim Android Play Store, iOS TestFlight/App Store, or physical-device readiness.

## Verification

- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/check-mobile-platform-package-preflight.sh`
- `bash scripts/plan-coverage.sh`
- `git diff --check`

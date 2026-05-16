# Platform Artifact Consumption Runbook - 2026-05-16

本报告记录平台 artifact 获取与手工 smoke 指引的整理结果。`docs/plan/` 未修改。

## Scope

- Code/docs scope: `docs/dev-runbook.md`, `scripts/check-release-baseline.sh`.
- Evidence basis: GitHub run `25960266472`, Docker release smoke, release/native guard scripts.
- Non-goal: 声明 signed release、store distribution、physical-device readiness、native process runtime 或 native authority writes。

## Changes

- Added `Platform Artifact Consumption` to `docs/dev-runbook.md`.
- Documented full shell-only target-host dispatch inputs for Desktop macOS/Windows, Android emulator, and iOS simulator.
- Documented evidence artifact validation through `scripts/collect-native-target-host-evidence.sh`.
- Documented package artifact download commands for:
  - `deve-desktop-macos-packages`
  - `deve-desktop-windows-packages`
  - `deve-mobile-android-packages`
  - `deve-mobile-ios-packages`
- Added explicit artifact interpretation boundaries for Docker, Desktop macOS, Desktop Windows, Android, and iOS.
- Updated `scripts/check-release-baseline.sh` to guard the new runbook section.

## Verification

Ran:

- `bash -n scripts/check-release-baseline.sh scripts/check-desktop-target-host-preflight.sh`
- `shellcheck scripts/check-release-baseline.sh scripts/check-desktop-target-host-preflight.sh`
- `scripts/check-release-baseline.sh`
- `git diff --check`

Results:

- All passed.

## Decision

Platform artifact consumption is now documented at the runbook layer. Current evidence remains shell-only:

- no native process runtime;
- no native authority writes;
- no signed macOS release claim;
- no signed Windows installer claim;
- no Android Play Store claim;
- no iOS TestFlight/App Store claim;
- no physical-device readiness claim.

Next useful batch is a post-platform mainline guard refresh to ensure the CI/script/runbook changes did not introduce drift outside release/platform surfaces.

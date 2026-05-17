# Full Regression Gate Refresh After Mainline Local Closures - 2026-05-17

本报告记录 Repo File Operations、Settings Local Feedback、Source Control Command Surface 三个本地主线批次后的全量回归闸门。`docs/plan/` 未修改。

## Scope

- Closed local batches: Repo File Operations, Settings Local Feedback, Source Control Command Surface.
- Purpose: before selecting the next feature implementation batch, verify workspace-wide compile/test/lint/runtime/coverage health.
- Boundaries kept closed: platform signing, physical-device release, native process runtime, Web Git writer, server-backed Settings API, native authority writes.

## Guards

Ran:

- `cargo fmt --check`
- `cargo test --locked`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-architecture-registry.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-settings-local-feedback-baseline.sh`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/check-repo-file-ops-baseline.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/check-network-baseline.sh`
- `bash scripts/check-search-baseline.sh`
- `bash scripts/check-rendering-baseline.sh`
- `bash scripts/check-ai-baseline.sh`
- `bash scripts/check-graph-baseline.sh`
- `bash scripts/check-mobile-baseline.sh`
- `bash scripts/check-browser-prefs-boundary.sh`
- `bash scripts/check-large-doc-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `bash scripts/plan-coverage.sh`
- `bash scripts/smoke-runtime-happy-path.sh`
- `bash scripts/smoke-runtime-recovery-path.sh`
- `git diff --check`

Results:

- Format: passed.
- Full workspace tests: passed.
- All-feature clippy: passed.
- Acceptance bindings: automated `117`, feature walkthrough `54`, manual `29`, unbound soft `0`.
- Architecture registry: `72` flows, active drift `0`.
- Feature operation paths: ok.
- Domain baselines: passed.
- Dev runbook baseline: passed after adding the missing Settings local feedback and Repo file operations guard entries.
- Plan coverage: blocking violations `0`, soft warnings `18`.
- Runtime happy/recovery smoke: passed.
- Diff hygiene: passed.

## Decision

The full regression gate is green for the current Web/server mainline.

Next batch: **Mainline Feature Implementation Selection After Full Regression**.

Rationale:

- The recent local feature closures did not introduce workspace-wide compile, lint, plan coverage, runtime smoke, or domain baseline drift.
- The next step should be a fresh plan/features/acceptance/code gap rescan before selecting the next implementation batch.
- Platform post-gate work remains deferred unless explicitly reopened after mainline gap selection.

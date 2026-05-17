# Mainline Gap Rescan After Repo File Operations Closure - 2026-05-17

本报告记录 Repo File Operations Closure 后的主线守卫复扫。`docs/plan/` 未修改。

## Scope

- Closed batch:
  - `docs/report/repo-file-operations-baseline-2026-05-17.md`
  - `docs/report/repo-file-operations-browser-smoke-2026-05-17.md`
- Goal: 确认本批新增 acceptance/script/report 没有引入 plan/code/docs drift，并选择下一批本地可推进主线功能。
- Non-goal: 平台 signing/device gate、native process runtime、native authority write、Web Git writer、server-backed Settings API。

## Mapping Guards

Ran:

- `bash scripts/plan-coverage.sh`
- `bash scripts/check-architecture-registry.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`

Results:

- `plan-coverage`: blocking violations `0`.
- `plan-coverage`: soft size warnings `18`, unchanged from prior scans.
- `plan_ref`: dangling blocking refs `0`.
- `i18n facade leak`: `0`.
- `acceptance bindings`: automated `113`, feature walkthrough `58`, manual `29`, unbound soft `0`.
- `architecture registry`: `72` flows, active drift `0`.
- `feature operation paths`: ok.

## Domain / Runtime Guards

Ran:

- `bash scripts/check-repo-file-ops-baseline.sh`
- `bash scripts/check-storage-repo-baseline.sh`
- `bash scripts/check-ui-desktop-baseline.sh`
- `bash scripts/smoke-runtime-happy-path.sh`
- `bash scripts/smoke-runtime-recovery-path.sh`
- `git diff --check`

Results:

- All checks passed.
- Repo File Operations Closure did not introduce mapping drift.
- No new unblocked Current MUST was identified by this focused rescan.

## Decision

Repo File Operations Closure is closed for current Web/server scope.

Next local feature batch: **Settings Local Persistence / Feedback Closure**.

Rationale:

- It is already described by `settings_persistence_apply.md`, `settings_update.md`, and `SET-003..006`.
- It has no external signing/device/target-host prerequisite.
- It must stay inside current file-backed config and browser UI prefs boundaries.
- Server-backed Settings API remains outside the current operation and must not be implemented in this batch.

## Next

Start with targeted baseline:

- `deve config print`
- `deve config set` whitelist / future-key rejection tests
- Settings UI immediate feedback tests
- reserved/disabled Settings feedback tests

Then run one browser smoke for Settings open -> locale/sync/backend feedback -> reload persistence where the current boundary supports it.

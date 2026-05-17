# Mainline Feature Gap Selection After Web PWA Manifest - 2026-05-17

本报告记录 Web PWA manifest runtime smoke 后的 Web/server 主线缺口选择。`docs/plan/` 未修改。

## Scope

- 闭合批次：`docs/report/web-pwa-manifest-contract-2026-05-17.md`。
- 运行态证据：`docs/report/web-pwa-manifest-browser-smoke-2026-05-17.md`。
- 复核范围：Web shell、dashboard/disconnect、mobile viewport、repo file operations、network、acceptance、feature path、plan coverage、runtime happy/recovery。
- 不打开 Web Git writer、server-backed Settings API、native process runtime、signing、physical-device 或 native authority writes。

## Verification

Ran:

- `bash scripts/check-ui-spa-routing-baseline.sh`
- `bash scripts/check-ui-dashboard-refresh-baseline.sh`
- `bash scripts/check-ui-disconnect-baseline.sh`
- `bash scripts/check-ui-desktop-baseline.sh`
- `bash scripts/check-mobile-baseline.sh`
- `bash scripts/check-repo-file-ops-baseline.sh`
- `bash scripts/check-network-baseline.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/plan-coverage.sh`
- `bash scripts/smoke-runtime-happy-path.sh`
- `bash scripts/smoke-runtime-recovery-path.sh`

Results:

- Web/UI/mobile baselines: passed.
- Repo file operations baseline: passed.
- Network baseline: passed.
- Feature operation paths: passed.
- Acceptance bindings: automated `149`, feature walkthrough `54`, manual `0`, unbound `0`.
- Plan coverage: blocking violations `0`, soft warnings `18`, dangling `plan_ref` `0`, i18n leaks `0`.
- Runtime happy/recovery smoke: passed.

## Findings

- No new blocking plan/code drift was found after Web PWA manifest closure.
- No small unblocked Current Web/server feature gap was identified that is safer than a regression gate refresh.
- Existing plan-coverage soft warnings remain cohesion warnings, not reasons for line-count-only splitting.
- PWA remains metadata-only: no service worker, offline authority, native runtime, or browser storage authority was introduced.

## Decision

Mainline feature gap selection after Web PWA manifest is closed.

Next batch: **Full Regression Gate Refresh After Web Shell Current Closure**.

Rationale:

- Recent Web shell current contracts were closed in small batches: Repo Switcher and PWA Manifest.
- Targeted domain guards are green, so the next useful step is full workspace regression rather than opening a larger gated feature.
- Platform work remains behind existing Desktop/Android post-gate boundaries until Web/server mainline is green under full regression.

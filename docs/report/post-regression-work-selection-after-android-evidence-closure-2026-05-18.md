# Post-regression Work Selection After Android Evidence Closure - 2026-05-18

本报告记录 Android shell-only target-host evidence closure 与 full regression gate 后的下一批工作选择。`docs/plan/` 未修改。

## Inputs

- `docs/report/full-regression-gate-refresh-after-android-target-host-evidence-closure-2026-05-18.md`
- `docs/report/mainline-gap-rescan-after-android-target-host-evidence-refresh-2026-05-18.md`
- `docs/report/android-target-host-evidence-refresh-after-desktop-installer-closure-2026-05-18.md`
- `docs/report/desktop-installer-target-host-evidence-refresh-2026-05-17.md`
- `docs/report/platform-evidence-refresh-after-web-shell-current-closure-2026-05-17.md`
- `docs/report/next-tasks.md`
- `docs/plan/08_ui_design_03_mobile.md`
- `docs/plan/14_tech_stack.md`
- `docs/plan/15_release.md`

## Guard Check

- `bash scripts/check-acceptance-bindings.sh`: automated `149`, feature walkthrough `54`, manual `0`, unbound `0`.
- `bash scripts/check-feature-operation-paths.sh`: ok.
- `bash scripts/check-architecture-registry.sh`: flows `72`, active drift `0`.
- `bash scripts/check-release-baseline.sh`: ok.
- `bash scripts/check-mobile-baseline.sh`: ok.
- `bash scripts/check-native-process-adapter-gate.sh`: ok.

## Candidate Review

- Current Web/server gap: latest full regression gate found no new unblocked Current Web/server `MUST` gap.
- Desktop scope: package、startup、native-session 与 installer target-host evidence 已闭合；继续扩大 Desktop runtime scope would open broader platform design work.
- Android scope: Android shell-only package/install/startup target-host evidence 已刷新到 `699e5bbd` and validated.
- iOS scope: latest iOS simulator shell package/install/startup evidence still comes from the earlier all-platform run `25980828117`; `08_ui_design_03_mobile.md#mobile-ios-shell-package-execution-gate` explicitly allows iOS shell-only package execution as an independent gate.

## Decision

Select Mobile iOS Target-host Evidence Refresh After Android Evidence Closure.

## Scope

- Dispatch Native Target Host with `target=mobile-ios`.
- Run required iOS preflight, shell package build, and simulator install/startup smoke.
- Collect and validate `mobile-ios.md` target-host evidence.
- iOS remains shell-only; Mobile process runtime stays closed.
- Signing、store、physical-device readiness、native authority writes、Android process runtime、Web Git writer 与 server-backed Settings API remain out of scope.

## Acceptance

- GitHub target-host run completes successfully for `mobile-ios`.
- Evidence includes `mobile_ios_preflight=success`, `process_gate=success`, `package_build=success`, and `install_startup_smoke=success`.
- Evidence states process runtime gate and native authority writes remain closed.
- Follow-up evidence report records run id, commit head, artifact validation, and platform boundary.

# Post-regression Work Selection After Desktop Native Session Evidence Closure - 2026-05-17

本报告记录 Desktop native-session evidence closure 与 full regression gate 后的下一批工作选择。`docs/plan/` 未修改。

## Inputs

- `docs/report/full-regression-gate-refresh-after-desktop-native-session-evidence-closure-2026-05-17.md`
- `docs/report/desktop-native-session-target-host-evidence-refresh-2026-05-17.md`
- `docs/report/next-tasks.md`
- `docs/plan/08_ui_design_02_desktop.md`
- `docs/plan/08_ui_design_03_mobile.md`
- `docs/plan/15_release.md`

## Candidate Review

- Current Web/server gap: latest full regression gate found no new unblocked Current Web/server `MUST` gap.
- Android target-host package evidence refresh: useful, but existing Android emulator package/install/startup evidence already covers shell-only package execution; Android process runtime and native authority writes remain closed.
- Desktop installer required smoke preflight: latest Desktop native-session target-host evidence has package build、startup smoke 与 native-session smoke green, but `installer_smoke=skipped`.

## Decision

Select Desktop Installer Required Smoke Preflight After Native Session Evidence.

## Scope

- Inspect and harden existing Desktop target-host installer smoke workflow, scripts, and evidence fields.
- Required mode MUST fail closed when installer artifact, target-host prerequisite, or required evidence field is missing.
- Desktop native-session authority remains unchanged.
- Android remains shell-only; Android process runtime stays closed.
- Signing、store、physical-device readiness、native authority writes、Web Git writer 与 server-backed Settings API remain out of scope.

## Acceptance

- Local diagnostic checks pass, or skip with an explicit non-authoritative reason.
- Target-host workflow can report installer smoke as success、skipped 或 blocker with structured evidence.
- Follow-up evidence report records the installer smoke result and preserves platform boundary statements.


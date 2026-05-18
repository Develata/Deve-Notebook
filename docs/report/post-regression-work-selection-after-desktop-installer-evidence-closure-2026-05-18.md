# Post-regression Work Selection After Desktop Installer Evidence Closure - 2026-05-18

本报告记录 Desktop installer evidence closure 与 full regression gate 后的下一批工作选择。`docs/plan/` 未修改。

## Inputs

- `docs/report/full-regression-gate-refresh-after-desktop-installer-evidence-closure-2026-05-18.md`
- `docs/report/mainline-gap-rescan-after-desktop-installer-target-host-evidence-2026-05-17.md`
- `docs/report/desktop-installer-target-host-evidence-refresh-2026-05-17.md`
- `docs/report/platform-evidence-refresh-after-web-shell-current-closure-2026-05-17.md`
- `docs/report/next-tasks.md`
- `docs/plan/08_ui_design_02_desktop.md`
- `docs/plan/08_ui_design_03_mobile.md`
- `docs/plan/15_release.md`

## Candidate Review

- Current Web/server gap: latest full regression gate found no new unblocked Current Web/server `MUST` gap.
- Desktop post-gate scope decision: Desktop package、startup、native-session 与 installer target-host evidence 已闭合；继续扩大 Desktop runtime scope would open broader platform design work.
- Android target-host package evidence refresh: existing Android emulator install/startup evidence is older than the latest Desktop target-host workflow hardening and current `HEAD`; refreshing Android shell-only evidence is the smallest platform follow-up.

## Decision

Select Android Target-host Package Evidence Refresh After Desktop Installer Evidence Closure.

## Scope

- Dispatch Native Target Host with `target=mobile-android`.
- Run required preflight, Android shell package build, and emulator install/startup smoke.
- Collect and validate `mobile-android.md` target-host evidence.
- Android remains shell-only; Android process runtime stays closed.
- Signing、store、physical-device readiness、native authority writes、Web Git writer 与 server-backed Settings API remain out of scope.

## Acceptance

- GitHub target-host run completes successfully for `mobile-android`.
- Evidence includes `mobile_android_preflight=success`, `process_gate=success`, `package_build=success`, and `install_startup_smoke=success`.
- Evidence states process runtime gate and native authority writes remain closed.
- Follow-up evidence report records run id, commit head, artifact validation, and platform boundary.

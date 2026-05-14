# Mobile Android Shell-only Package Gate

Date: 2026-05-14

## Scope

Opened a narrow plan gate for Android shell-only package execution.

No Android project generation or package build was run in this batch.

## Result

- `08_ui_design_03_mobile.md` now splits Mobile packaging into dependency
  spike, Android shell-only package execution, iOS package execution, and
  process runtime layers.
- `14_tech_stack.md` now delegates Android shell-only package execution to
  `08_ui_design_03_mobile.md#mobile-android-shell-package-execution-gate`.
- `docs/plan/AGENTS.md` registers the new stable anchor.
- `scripts/check-native-track-boundary.sh` guards the new Android shell-only
  authority/process boundary text.

## Boundary

- Android package execution may proceed only as shell-only target-host work.
- iOS package execution remains blocked on macOS target-host evidence.
- Mobile process runtime remains closed.
- Native authority writes remain closed.
- Default workspace builds remain no-Tauri/no-process.

## Verification

- `scripts/check-native-track-boundary.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `git diff --check`

## Follow-up

Implement the Android shell-only package execution batch next. That batch may
add the Android target-host script and Tauri Android package execution surface,
but it must not open iOS, process runtime, or native authority writes.

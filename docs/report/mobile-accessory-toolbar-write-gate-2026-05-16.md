# Mobile Accessory Toolbar Write Gate - 2026-05-16

本报告记录移动辅助键盘栏写入门禁闭合。`docs/plan/` 未修改。

## Scope

- Code scope: `apps/web/src/components/mobile_layout/`, `apps/web/index.html`.
- Docs/guard scope: `docs/features/08_ui_design_03_mobile.md`, `docs/acceptance-cases/05_ui.md`, `docs/acceptance-bindings.tsv`, `scripts/check-mobile-baseline.sh`.
- Non-goal: 改动 editor authority、打开 native process runtime、打开 native authority writes。

## Changes

- Mobile accessory toolbar `readonly` now derives from the full repo write gate through `repo_write_allowed_for_core_tracked`.
- Toolbar insert/wrap/undo callbacks now re-check the readonly signal before calling JS FFI.
- Undo is disabled together with all other accessory toolbar buttons.
- JS mobile editor helpers now reject insert/wrap/undo when CodeMirror state is read-only.
- Added `UI-MOB-019` acceptance binding and mobile baseline guards.

## Verification

Ran:

- `cargo fmt --check`
- `cargo test -p deve_web mobile_toolbar_write_gate -- --nocapture`
- `scripts/check-mobile-baseline.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/plan-coverage.sh`
- `git diff --check`

Results:

- All passed.
- Acceptance bindings: `109` automated, `60` feature walkthrough, `29` manual, `0` unbound soft cases.
- Plan coverage: `0` blocking violations, `17` existing soft file-size warnings.

## Decision

Mobile accessory toolbar write actions now share the same write boundary as the editor. This closes the selected P1 gap from the post-platform scan.

# Modal Focus Contract Closure - 2026-05-16

本报告记录 Modal Focus Contract Closure 批次。`docs/plan/` 未修改。

## Scope

- Settings modal.
- Pending Navigation modal.
- Merge modal.
- Existing Command Palette and Search focus behavior remained unchanged.

## Changes

- Added shared `focus_scope::attach_modal_focus_restore_effect` for non-input modal surfaces.
- Settings, Pending Navigation, and Merge modals now expose `role="dialog"`, `aria-modal="true"`, `tabindex="-1"`, and shared Tab trap handling.
- The same modals now capture the previous focus target on open and restore it on close, falling back to `.cm-content`.
- `UI-GEN-003` and `scripts/check-ui-focus-baseline.sh` now guard the expanded shared modal contract.

## Verification

Ran:

- `cargo fmt`
- `cargo fmt --check`
- `scripts/check-ui-focus-baseline.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-release-baseline.sh`
- `scripts/plan-coverage.sh`
- `git diff --check`
- `cargo test -p deve_web focus_scope -- --nocapture`

Results:

- UI focus baseline: pass.
- Feature operation paths: pass.
- Acceptance bindings: `103` automated, `60` feature walkthrough, `29` manual, `0` unbound soft cases.
- Release baseline: pass.
- Plan coverage: `0` blocking violations, `17` existing soft file-size warnings.
- Focus scope tests: pass.
- Format and diff hygiene: pass.

## Decision

Modal Focus Contract Closure is closed. Next executable work is Storage/Server Edge Coverage.

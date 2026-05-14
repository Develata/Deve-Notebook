# Error Code Catalog Plan Alignment - 2026-05-14

本报告记录一次小范围 `docs/plan/11_i18n.md` catalog 对齐。变更仅限错误码目录，不调整 plan 骨架、术语或其它章节。

## Scope

- Add missing source-control error code entry.
- Add missing graph error code entry.
- Keep `11_i18n.md#i18n-error-code-catalog` as the single authoritative catalog.

## Changes

- Added `SC_COMMIT_DIFF_UNPROJECTABLE`.
- Added `GRAPH_DEGRADED_PROJECTION_REQUIRED`.

## Verification

Ran:

- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `scripts/check-i18n-hardcoded-baseline.sh`
- `scripts/check-i18n-formatting-baseline.sh`
- `cargo test -p deve_core serde_ -- --nocapture`
- `cargo test -p deve_cli graph_projection_degraded_error_maps_to_structured_code -- --nocapture`
- `cargo test -p deve_cli commit_diff -- --nocapture`
- `cargo test -p deve_web source_control_graph -- --nocapture`

Results:

- Plan coverage: `0` blocking violations, `17` existing soft file-size warnings.
- I18N baselines: pass.
- Error-code serde stability tests: pass.
- Graph degraded structured error mapping: pass.
- Source Control commit diff mapping tests: pass.
- Graph panel i18n copy test: pass.

## Decision

The known plan-side error-code catalog drift is closed. Further `docs/plan/` changes remain gated by explicit authorization.

# Remote Ops Batch Apply Failure Fallback

Date: 2026-05-16

## Scope

- Code scope: Web editor remote op batch replay and snapshot delta fallback.
- Docs/guard scope: `RENDER-LARGE-002`, large-doc operation notes, and `scripts/check-large-doc-baseline.sh`.
- Plan source: `docs/plan/03_rendering.md` recovery contract for failed snapshot delta application.

## Result

- `applyRemoteOpsBatch` now returns success/failure instead of swallowing JS exceptions.
- Remote op batch replay validates editor ranges before dispatch.
- Failed batch replay stops incremental replay and does not advance local version/history.
- Matching snapshot scope failures trigger a full `OpenDoc` fallback path.
- Pending overlay replay failures are logged instead of being silently ignored.

## Boundary

- This does not add a second editor authority buffer.
- This does not change the wire protocol.
- Full snapshot fallback reuses the existing `OpenDoc` path and scope gate.

## Verification

- `cargo test -p deve_web snapshot_apply_failure -- --nocapture`
- `scripts/check-large-doc-baseline.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/plan-coverage.sh`

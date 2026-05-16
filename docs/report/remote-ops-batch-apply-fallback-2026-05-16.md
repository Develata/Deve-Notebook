# Remote Ops Batch Apply Failure Fallback

Date: 2026-05-16

## Scope

- Code scope: Web editor remote op batch replay and snapshot delta fallback.
- Docs/guard scope: `RENDER-LARGE-002`, large-doc operation notes, and `scripts/check-large-doc-baseline.sh`.
- Plan source: `docs/plan/03_rendering.md` recovery contract for failed snapshot delta application.

## Result

- `applyRemoteOpsBatch` now returns success/failure instead of swallowing JS exceptions.
- Remote op batch replay validates editor ranges before dispatch.
- Failed batch replay stops incremental replay and does not advance local version/history until fallback succeeds.
- Matching snapshot scope failures first apply reconstructed full snapshot content locally.
- `OpenDoc` reopen is retained only as a last-resort fallback when local reconstruction fails.
- Pending overlay replay failures are logged instead of being silently ignored.
- Adapter-not-ready batch calls now fail closed instead of reporting queued work as synchronous success.

## Boundary

- This does not add a second editor authority buffer.
- This does not change the wire protocol.
- Full snapshot fallback uses `deve_core::state::try_apply_content_ops` to build the full content from snapshot base plus delta ops.
- Snapshot reopen reuses the existing `OpenDoc` path and scope gate only when local full-content reconstruction fails.

## Verification

- `cargo test -p deve_core try_apply_content_ops -- --nocapture`
- `cargo test -p deve_web snapshot -- --nocapture`
- `cargo test -p deve_web snapshot_apply -- --nocapture`
- `npm --prefix apps/web run build`
- `cargo test -p deve_web snapshot_apply_failure -- --nocapture`
- `bash scripts/check-large-doc-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/plan-coverage.sh`

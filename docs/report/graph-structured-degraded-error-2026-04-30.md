# Graph Structured Degraded Error - 2026-04-30

## Status

Graph degraded projection detection no longer depends on CLI/user-facing error
text.

Implemented:

- Added `ServerErrorCode::GraphDegradedProjectionRequired` serialized as
  `GRAPH_DEGRADED_PROJECTION_REQUIRED`.
- Added a typed `GraphProjectionError::DegradedProjectionRequired` in the shared
  CLI/HTTP graph projection adapter.
- HTTP graph projection maps that typed error to the structured server error
  code.
- Web graph API now classifies degraded projection by `ServerErrorCode`, not by
  searching `detail` for `--allow-degraded-projection`.

## Boundary

- CLI output still keeps the operator hint for `--allow-degraded-projection`.
- Web still does not auto-retry with `allow_degraded_projection=true`.
- Graph projection remains read-only and does not gain authority writes.

## Verification

- `cargo test -p deve_core protocol -- --nocapture`
- `cargo test -p deve_cli graph -- --nocapture`
- `cargo test -p deve_web graph -- --nocapture`
- `scripts/check-graph-baseline.sh`
- `git diff --check`

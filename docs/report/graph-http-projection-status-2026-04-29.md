# Graph HTTP Projection Status - 2026-04-29

## Status

P3-13 graph data surface advanced from CLI-only projection export to shared
CLI/HTTP read-only projection.

Current implementation:

- `apps/cli/src/graph_projection.rs` is the shared read-only adapter used by
  `deve graph` and server HTTP.
- `GET /api/repo/graph` is a protected HTTP query route.
- The endpoint returns `deve_core::graph::GraphProjection` JSON for the selected
  local repo.
- Default behavior remains fail-closed when Structure Facts authority is
  corrupt; explicit `allow_degraded_projection=true` is required for metadata
  fallback.

## Boundary

- No graph endpoint writes ledger, workspace, search index, source-control
  tables, `.git`, or `.notegit`.
- No Web graph renderer was introduced in this batch.
- Canvas / d3-force / Pixi.js visualization remains future renderer work.

## Verification

- `cargo test -p deve_cli graph -- --nocapture`
- `scripts/check-graph-baseline.sh`

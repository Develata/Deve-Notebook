# Graph Web Projection Panel Status - 2026-04-29

## Status

P3-13 graph now has a minimal Web read-only projection panel.

Implemented:

- `apps/web/src/api/graph.rs` reads protected `GET /api/repo/graph`.
- Source Control `Graph` section now renders `GraphPanel` instead of reusing
  commit history UI.
- The panel displays repo-scoped `nodes`, `edges`, and `unresolved` counts.
- The panel handles loading, failed, empty, and local-only blocked states.

## Boundary

- No d3-force, Pixi.js, Canvas renderer, layout engine, or graph interaction was
  added.
- No Web write path was added.
- Graph remains a read-only projection derived from current repo docs.

## Verification

- `cargo test -p deve_web graph -- --nocapture`
- `cargo test -p deve_web source_control -- --nocapture`
- `cargo check --workspace --all-targets --all-features`

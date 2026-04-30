# Graph Blocked/Degraded Acceptance Polish - 2026-04-30

## Status

P3-13 Graph summary panel acceptance polish is closed.

Implemented:

- Web Graph panel now distinguishes `local-only`, `blocked`, `degraded`, `empty`, and
  `error` states with stable `data-deve-graph-state` attributes.
- Remote/shadow branch scope is treated as `local-only`; Source Control read-gate
  failures are treated as `blocked` and include the gate reason.
- HTTP graph projection failures that require explicit
  `--allow-degraded-projection` are classified as `degraded` instead of generic
  request failure.
- The panel remains read-only and still does not introduce Canvas, d3-force,
  Pixi, layout worker, interaction state, or renderer dependencies.

## Boundary

- Web does not automatically retry with `allow_degraded_projection=true`.
- Degraded graph export remains an explicit CLI/operator decision.
- Graph remains a derived projection and does not write ledger, workspace,
  source-control state, search index, `.git`, or `.notegit`.

## Verification

- `cargo test -p deve_web graph -- --nocapture`
- `cargo test -p deve_web source_control -- --nocapture`
- `cargo test -p deve_cli graph -- --nocapture`
- `scripts/check-graph-baseline.sh`
- `scripts/plan-coverage.sh`
- `git diff --check`

## Next

The active short queue is empty. Before starting a new implementation batch,
re-read the latest drift report and plan chapters, then select the next narrow
P0/P1 item.

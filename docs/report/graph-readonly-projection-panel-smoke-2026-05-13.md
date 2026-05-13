# Graph Readonly Projection Panel Smoke - 2026-05-13

本报告记录 `Graph read-only projection panel browser smoke`。`docs/plan/` 仍是唯一权威；本文件只记录执行结果。

## Scope

- `docs/plan/14_tech_stack.md#graph-visualization`
- `docs/features/07_diff_logic.md` §3 Diff / History / Graph
- `docs/acceptance-cases/12_tech_release.md` TECH-GRAPH / Graph assertions

## Environment

- Frontend build: rebuilt with `scripts/smoke-web-release-build.sh`.
- Backend: `DEVE_LEDGER_DIR=/tmp/deve-graph-smoke-20260513-BEm3yf/ledger DEVE_VAULT_PATH=/tmp/deve-graph-smoke-20260513-BEm3yf/vault DEVE_STATIC_DIR=apps/web/dist cargo run -p deve_cli --bin deve_cli -- serve --dev --port 32026`
- Browser URL: `http://127.0.0.1:32026/?isolatedContext=deve-graph-smoke`
- Data root: `/tmp/deve-graph-smoke-20260513-BEm3yf`
- Chrome MCP viewport: desktop `1280x900`
- Auth: development defaults, existing authenticated session.

## Results

Passed:

- Empty local repo graph rendered as readonly summary:
  - `data-deve-graph-panel="readonly"`
  - `data-deve-graph-projection-mode="readonly-summary"`
  - `data-deve-graph-renderer-gate="closed"`
  - `data-deve-graph-state="empty"`
  - `nodes=0`, `edges=0`, `unresolved=0`
- Protected HTTP graph endpoint returned read-only projection JSON:
  - `GET /api/repo/graph` returned `200`.
  - response keys were `nodes`, `edges`, `unresolved_links`.
- After creating and editing a minimal document through Web UI, Graph re-fetch rendered loaded summary:
  - `data-deve-graph-state="loaded"`
  - `data-deve-graph-stat="nodes"` with value `1`
  - `data-deve-graph-stat="edges"` with value `1`
  - `data-deve-graph-stat="unresolved"` with value `1`
- Renderer gate remained closed while loaded:
  - `data-deve-graph-renderer-gate="closed"`
  - visible copy stated Canvas rendering remains future work.
- Intentional server stop plus re-opened Graph section rendered blocked state:
  - `data-deve-graph-state="blocked"`
  - panel copy explained Source Control read scope was not ready.
- Browser-degraded UI branch rendered correctly when `/api/repo/graph` returned structured `GRAPH_DEGRADED_PROJECTION_REQUIRED`:
  - `data-deve-graph-state="degraded"`
  - panel copy directed degraded export to explicit CLI `--allow-degraded-projection`.
- Stable final reload had no browser console `error` or `warn`.
- Stable final reload network requests returned `200` or `304`; `/api/repo/graph?...` returned `200`.

Expected during the intentional disconnect:

- Browser emitted normal WebSocket close / connection refused diagnostics while the server was stopped. These were not present after the final stable reload.

## Fixed Gaps

- Added explicit Graph panel markers for readonly summary mode and closed renderer gate.
- Added stable stat value marker for graph summary counts.
- Extended `scripts/check-graph-baseline.sh` to guard these markers.
- No `docs/plan/` files were changed.

## Verification

已运行：

- `bash scripts/check-graph-baseline.sh`
- `cargo test -p deve_web graph -- --nocapture`
- `cargo test -p deve_cli graph -- --nocapture`
- `scripts/smoke-web-release-build.sh`
- Chrome MCP browser smoke as described above

结果：

- Graph baseline guard: pass
- Web graph tests: pass, 7 tests
- CLI/server graph tests: pass, 6 tests
- Web release build: pass
- Browser Graph readonly projection smoke: pass

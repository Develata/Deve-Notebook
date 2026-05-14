# Source Control HTTP Scope Gate 2026-05-14

## Scope

- Closed the `/api/sc/*` HTTP scope nonce gap without changing `docs/plan/`.
- Kept the current Web Source Control writer path on WebSocket.
- Preserved plugin-host proxy behavior by making the proxy explicitly carry a source-control HTTP scope nonce.

## Changes

- Added `source_control::http_scope` as the HTTP source-control scope gate.
- `/api/sc/*` read and mutation handlers now reject missing or zero `scope_nonce` with structured `SC_REPO_CONTEXT_INVALID`.
- `RemoteSourceControlApi` now sends `scope_nonce` on query and mutation calls.
- Web Git mirror readonly repair review now includes the current scope nonce and discards late results after scope changes.
- Source-control baseline now guards the HTTP scope gate and Web repair-review scope propagation.

## Boundary

The HTTP layer is stateless and does not own the WebSocket per-connection scope authority. This batch enforces explicit HTTP scope input and client-side stale result suppression; WebSocket remains the authority path for live source-control state transitions.

## Verification

- `cargo test -p deve_cli source_control_http -- --nocapture`
- `cargo test -p deve_web git_mirror -- --nocapture`
- `cargo test -p deve_web git_repair_command_sets_cli_only_notice -- --nocapture`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`
- `cargo fmt --check`

# WS Text Frame Debug Gate - 2026-05-14

本报告记录 WebSocket text-frame runtime contract 修复。`docs/plan/` 未修改。

## Scope

- Plan basis: `05_network.md#protocol-versioning`.
- Code scope: server WS receive path, WS protocol acceptance tests, network baseline guard.
- Non-goal: remove protocol-layer JSON debug decoder, change binary frame schema, or change browser binary default.

## Changes

- Server receive path now gates all decoded JSON text frames before routing.
- Production default rejects both versioned JSON text and legacy JSON text with structured `ProtocolError`.
- Explicit debug compatibility remains available through `DEVE_ENV=development`, `DEVE_ALLOW_WS_JSON_TEXT=1`, or legacy alias `DEVE_ALLOW_LEGACY_WS_JSON=1`.
- Network baseline guard now checks the unified JSON text gate instead of legacy-only gating.
- WS acceptance now verifies current-version JSON text is rejected by default.

## Verification

Ran:

- `cargo fmt --check`
- `cargo test -p deve_cli ws_endpoint_rejects_versioned_json_text_by_default -- --nocapture`
- `cargo test -p deve_cli ws_endpoint_rejects_legacy_json_text_by_default -- --nocapture`
- `cargo test -p deve_cli ws::receive -- --nocapture`
- `bash scripts/check-network-baseline.sh`

Results: pass.

## Residual Work

- Mobile PendingAck scope filtering remains next.
- Source Control HTTP scope gate remains a larger runtime-design followup.

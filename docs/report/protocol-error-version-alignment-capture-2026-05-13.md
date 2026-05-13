# Protocol Error / Version Alignment Capture - 2026-05-13

本报告记录 `NET-004/012/013` 相关协议错误与版本对齐批次。`docs/plan/` 仍是唯一权威；本文件只记录当前实现事实与本批修复。

## Scope

- `docs/plan/05_network.md`
- `docs/plan/16_web_thin_client_ledger.md`
- `docs/acceptance-cases/06_network.md`
- `crates/core/src/protocol/frame.rs`
- `apps/cli/src/server/ws/receive/`
- `apps/web/src/api/incoming/`

## Findings

Confirmed:

- WebSocket binary frame magic remains `DEVEWSF3`.
- `WS_PROTOCOL_VERSION` and `MIN_SUPPORTED_WS_PROTOCOL_VERSION` remain `8`.
- Legacy JSON text is still accepted only through explicit development/debug compatibility policy.
- Binary frames missing `DEVEWSF3` magic remain rejected.
- Browser auth failure routing remains outside generic reconnect/error classification.

Fixed:

- Server-side unsupported WS protocol versions now return `SYNC_VERSION_MISMATCH`, not generic `REQUEST_FAILED`.
- Server-side malformed WS frame payloads now return `SYNC_INVALID_PAYLOAD`.
- Server-side versioned JSON text with unsupported protocol version is covered by regression tests.
- Server-side malformed versioned binary payload is covered by regression tests.
- Web-side malformed versioned server binary payload now becomes a local `ProtocolError` with `SYNC_INVALID_PAYLOAD`.
- Web-side raw legacy binary without magic remains ignored instead of confirming a connection.
- Network and structured-error guard scripts now pin the version/error mapping.

## Verification

Ran:

- `cargo test -p deve_cli receive -- --nocapture`
- `cargo test -p deve_web incoming -- --nocapture`
- `cargo test -p deve_core frame -- --nocapture`
- `bash scripts/check-network-baseline.sh`
- `bash scripts/check-ws-structured-errors.sh`

Results:

- Core frame protocol tests: pass.
- CLI receive protocol/error tests: pass.
- Web incoming decode tests: pass.
- Network baseline guard: pass.
- Structured WS error guard: pass.

## Decision

Protocol error / version alignment capture is closed. Continue with the final regression gate.

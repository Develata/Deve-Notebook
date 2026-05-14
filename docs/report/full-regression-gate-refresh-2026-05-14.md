# Full Regression Gate Refresh - 2026-05-14

本报告记录 Chrome MCP smoke 与 fresh gap scan 之后的全量回归闸门刷新。`docs/plan/` 未修改。

## Scope

- Full workspace Rust tests.
- Clippy across all targets.
- Runtime happy/recovery smoke scripts.
- Formatting, diff hygiene, and plan-code coverage.

## Fixes During Gate

- Updated Web protocol-error chat tests to assert localized `ServerErrorCode` copy (`Request failed`) instead of backend detail text.
- Limited Web `parse_ws_port` helper to `wasm32` and `test`, preserving browser behavior while avoiding native clippy dead-code failure.

## Verification

Ran:

- `cargo test -p deve_web message_dispatch_protocol -- --nocapture`
- `cargo test -p deve_web query_ws_port -- --nocapture`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo fmt --check`
- `git diff --check`
- `scripts/plan-coverage.sh`
- `scripts/smoke-runtime-happy-path.sh`
- `scripts/smoke-runtime-recovery-path.sh`

Results:

- Targeted Web protocol-error tests: pass.
- Targeted Web WS port parser test: pass.
- Full workspace `cargo test`: pass.
- Clippy: pass.
- Formatting and diff hygiene: pass.
- Plan coverage: 0 blocking violations, 17 soft file-size warnings.
- Runtime happy-path smoke: pass.
- Runtime recovery smoke: pass.

## Decision

The full regression gate is green. If `docs/plan/` remains closed and native packaging remains gated, the next batch should be a fresh mainline gap scan or a user-selected non-platform Current MUST.

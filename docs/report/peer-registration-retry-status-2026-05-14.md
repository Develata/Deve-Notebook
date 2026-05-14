# Peer Registration Retry Status

日期：2026-05-14

## Scope

本批次执行 `Mainline implementation gap scan` 后选择一个非平台化小缺口：Web UI 需要区分 session token 与 peer identity / writer-ready 状态，并允许用户显式重试 peer registration。

## Baseline

Ran:

- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/check-ws-structured-errors.sh`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-source-control-baseline.sh`

Results:

- Plan coverage: `0` blocking violations, `17` known soft size warnings.
- Acceptance bindings: `0` unbound cases.
- Feature path, auth, WS structured errors, native boundary, native packaging, release, source-control guards: pass.

## Fix

- `SyncStatusKind` 新增 `PeerNotRegistered`，把 `Connected + session valid + repo loaded` 但 writer/handshake 未完成的状态与 repo switching handshake 分开。
- Desktop bottom bar 与 mobile footer 显示 `Logged in / Peer not registered` / `已登录 / Peer 未注册`。
- Desktop bottom bar 与 mobile footer 增加 `Retry peer` / `重试 Peer` 入口。
- Retry action 清空 stale writer-ready、handshake scope、repo/doc/tree request id，并 bump `handshake_retry_nonce`。
- Handshake effect 订阅 `handshake_retry_nonce`，变化时重置 `last_mode`，允许同一 repo/scope 重新发起 registration。
- `check-network-baseline.sh` 增加 peer registration retry guard。

## Verification

- `cargo test -p deve_web peer_registration -- --nocapture`
- `cargo test -p deve_web status_summary -- --nocapture`
- `cargo test -p deve_web handshake -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p deve_web --all-targets -- -D warnings`
- `bash scripts/check-network-baseline.sh`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`

## Remaining

- 不修改 `docs/plan/`：错误码目录补丁仍需明确允许改 plan。
- 不打开 native packaging gate：Desktop/Mobile 继续保持 no-packaging skeleton。
- Watcher lifecycle、modal shell unification、peer identity 更完整的重试 UX 仍属于较大 followup，不混入本批次。

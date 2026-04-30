# Native Shell Baseline - 2026-05-01

本报告合并 Desktop/Mobile native adapter、server launch、service supervisor、readiness/recovery 与 packaging gate 的短状态报告。它只记录当前 native track 停靠点。

## Current Boundary

- 默认构建保持 no-Tauri skeleton。
- `apps/desktop` 与 `apps/mobile` 只承载 endpoint/session/bootstrap/readiness/recovery contract。
- Web native bootstrap 必须使用注入 endpoint/session；无效 bootstrap fail-closed，不回退端口猜测。
- Native service supervisor 固定 Starting、EndpointHealthy、SessionHandoffReady、Restarting、Offline 等状态与 retry budget。
- process adapter 与真实 child-process runtime deferred；默认构建不得 spawn、持有或重启后端子进程。
- `native-packaging` 仍是 future gate；真实 `tauri` / `tauri-build` dependency 不进入 workspace default build。
- Native UI readiness 必须同时考虑 auth、node role、repo handshake、writer-ready 与 current scope。

## Verified Surfaces

- Core native adapter 状态、事件、endpoint、session、readiness contract。
- Hidden `serve --native-loopback` native-safe launch surface。
- Desktop/mobile shell skeleton 的 recovery bootstrap 与 foreground/resume reprobe。
- Web header、bottom bar、mobile footer、overlay、Source Control gate 的 native recovery 状态。
- `scripts/check-native-track-boundary.sh` 守住 no-Tauri dependency/import、no process runtime leak 与 native shell tests。
- Recovery bootstrap 接受可选 `service_state`：`service_offline` 映射为 `NativeServiceOffline`，`foreground_reprobe` 映射为 `NativeReprobeRequired`，`session_invalid` 映射为 `Unauthorized`。
- Desktop/mobile shell skeleton 可以发出不含 endpoint secrets、session material、service failure reasons 的 recovery bootstrap payload。
- `ForegroundReprobe` 在 auth、node role、repo handshake、writer readiness 与当前 `scope_nonce` 重新校验前不可写。
- Native recovery 验证覆盖 `cargo test -p deve_desktop`、`cargo test -p deve_mobile`、`cargo test -p deve_web native_bootstrap -- --nocapture`、`cargo test -p deve_web status_summary -- --nocapture`、`cargo test -p deve_web write_gate -- --nocapture`、`cargo test -p deve_web connection_urls -- --nocapture`、`cargo check --locked -p deve_web --target wasm32-unknown-unknown`。

## Retired Source Reports

- `native-adapter-contract-status-2026-04-29.md`
- `native-packaging-dependency-gate-2026-04-29.md`
- `native-packaging-dependency-gate-decision-2026-04-29.md`
- `native-packaging-gate-recheck-2026-04-30.md`
- `native-plan-post-gate-wording-split-2026-04-30.md`
- `native-process-adapter-decision-2026-04-29.md`
- `native-server-launch-status-2026-04-29.md`
- `native-service-supervisor-status-2026-04-29.md`
- `native-shell-parity-review-2026-04-29.md`
- `native-web-bootstrap-status-2026-04-29.md`
- `native-web-recovery-status-2026-04-29.md`
- `desktop-native-shell-status-2026-04-29.md`
- `desktop-packaging-scaffold-status-2026-04-29.md`
- `desktop-runtime-readiness-status-2026-04-29.md`
- `mobile-native-shell-status-2026-04-29.md`
- `mobile-packaging-scaffold-status-2026-04-29.md`

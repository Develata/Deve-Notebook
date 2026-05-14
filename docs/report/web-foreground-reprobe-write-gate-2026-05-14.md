# Web Foreground Reprobe Write Gate

日期：2026-05-14

## 目标

浏览器从后台/失焦恢复到前台时，不再信任恢复前的 repo handshake 与 writer-ready 状态。

## 改动

- `WsService` 新增 foreground reprobe 状态 API：清空 writer-ready，标记 node-role 需要重新探测。
- Web handshake effect 监听 `visibilitychange`、`focus`、`blur`，仅在 `inactive -> active` 且 WS 已连接时触发 reprobe。
- Foreground reprobe 会清空 handshake retry key、request id、`handshake_ready` 与 `handshake_scope_nonce`，强制重新完成 repo handshake。
- Node-role probe 复用 `/api/node/role` 探测逻辑，并以 endpoint 与 connection epoch 防止 stale probe 结果污染当前连接。
- `check-native-track-boundary.sh` 增加 foreground reprobe guard。

## 验证

- `cargo test -p deve_web foreground_reprobe -- --nocapture`
- `cargo test -p deve_web node_role_probe -- --nocapture`
- `cargo test -p deve_web native_runtime_readiness -- --nocapture`
- `cargo test -p deve_web handshake -- --nocapture`
- `cargo clippy -p deve_web --all-targets -- -D warnings`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/plan-coverage.sh`

## 结论

本批次不修改 `docs/plan/`。Web foreground recovery 已对齐 native/mobile lifecycle contract 的写入门禁语义：旧 writer-ready 不会跨后台恢复继续授权写入，必须重新完成 node-role probe、repo handshake 与 writer-ready。

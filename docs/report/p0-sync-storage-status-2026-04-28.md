# P0 Sync / Browser Storage 状态 - 2026-04-28

## 结果

- Sync vector wire contract 已作为当前协议合同实现。
- Browser storage degraded write boundary 已实现，并有定向测试覆盖。

## Sync Wire Contract

- 当前二进制 WS 协议为 `DEVEWSF3`，`protocol_version = 3`。
- 新 `SyncRequest` 与 `SyncSnapshotRequest` frame 携带 `known_vector`。
- 新 `SyncPushSnapshot` frame 携带 `server_vector`。
- legacy JSON debug frame 可缺省这些字段并按空向量解析；新的 binary/runtime frame 必须显式发送。

## Browser Storage 边界

- JS bridge 中 WebCrypto Ed25519 key 使用 `extractable: false` 生成。
- 缺少 WebCrypto / IndexedDB / Ed25519 capability 时进入 `DegradedSyncMode`。
- degraded mode 保持只读，并阻止 writer registration 与 sync writes。
- output queue 将 `RegisterWriter`、`SyncPush`、`SyncPushSnapshot` 归类为 write message。

## 验证

- `cargo test -p deve_core frame -- --nocapture`
- `cargo test -p deve_cli sync_hello -- --nocapture`
- `cargo test -p deve_cli sync_transfer -- --nocapture`
- `cargo test -p deve_web storage_capabilities -- --nocapture`
- `cargo test -p deve_web output_write_classification -- --nocapture`

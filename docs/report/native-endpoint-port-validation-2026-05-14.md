# Native endpoint port validation

日期：2026-05-14

## 结论

Native endpoint validator 现在拒绝无效、越界或为 `0` 的端口。

## 变更

- `validate_native_endpoint_bases` 通过 `validate_port` 校验显式端口。
- 保留无端口 loopback base URL 的合法性。
- Web native bootstrap 复用同一 validator，非法端口进入 `InvalidEndpoint` blocker。
- Native boundary baseline 绑定 core validator 与 Web bootstrap 测试。

## 验证

- `cargo test -p deve_core native_endpoint_validation -- --nocapture`
- `cargo test -p deve_web rejects_native_bootstrap_with_invalid_or_zero_port -- --nocapture`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/plan-coverage.sh`
- `cargo fmt --check`
- `git diff --check`

# Desktop Tauri Bootstrap Injection Gate

日期：2026-05-17

## 范围

- 只覆盖 Desktop `native-packaging` 的 Tauri WebView bootstrap 注入面。
- 不打开 Android process runtime、native authority writes、Web Git writer、signing/store、physical-device readiness 或 server-backed Settings API。
- 不修改 `docs/plan/`。

## 结果

- 新增 `tauri_bootstrap` 接入层，将 Desktop local-service bootstrap 转换为 Tauri `js_init_script` 原始 JS source。
- `script_tag()` 继续保留给 HTML shell surface；Tauri 只接收无 `<script>` 标签的 `script_source()`。
- Tauri entrypoint 在显式 `DEVE_DESKTOP_LOCAL_SERVICE` opt-in 时尝试 local-service bootstrap。
- 成功路径只在 session 已绑定时注入 `http_base`、`ws_base`、`node_role` 与 `session_bound=true`。
- 失败路径只注入 recovery bootstrap：`service_offline` 或 `session_invalid`。
- 注入 source 拒绝 `token`、`secret`、`localStorage`、`location.href`、`AUTH_PASS`、`AUTH_SECRET` 等 forbidden material。
- 成功 runtime 只作为 `DesktopLocalServiceTauriState` 被 Tauri 管理，用于保持子进程生命周期；该 state 不授权 ledger/vault/source-control/search/Git 写入。

## 保留缺口

- 真正的 Desktop HttpOnly session material bridge 尚未实现。
- 当前不允许通过 URL、localStorage、日志、crash report、匿名 localhost bypass 或 dev secret 完成会话交接。
- 下一批应设计并实现 native-only session issuance/cookie install bridge；在该通道完成前，生产桌面 local-service 会话失败必须停留在 `session_invalid` recovery。

## 验证

- `cargo test --locked -p deve_desktop --features native-packaging tauri_bootstrap -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging tauri_entry -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging service_bootstrap -- --nocapture`
- `cargo check --locked -p deve_desktop --features native-packaging`
- `cargo clippy --locked -p deve_desktop --all-targets --features native-packaging -- -D warnings`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/plan-coverage.sh`
- `git diff --check`

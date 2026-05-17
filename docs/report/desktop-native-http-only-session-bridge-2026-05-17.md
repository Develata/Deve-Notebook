# Desktop Native HttpOnly Session Bridge - 2026-05-17

本报告记录 Desktop `native-packaging` local-service session material bridge。本批未修改 `docs/plan/`。

## Scope

- Source of truth: `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/09_auth.md`.
- Code scope: Desktop local-service entrypoint/bootstrap/Tauri bootstrap、CLI native-only auth endpoint、native gate scripts。
- Boundary: Desktop `native-packaging` only。

## Implemented

- Desktop parent 为每次 local-service spawn 生成一次性高熵 session bootstrap secret。
- Secret 只通过受控 child-process env 传给 `deve_cli serve --native-loopback`，并通过 custom `Debug` redaction 防止日志泄漏。
- CLI server 仅在 native loopback launch 且 secret 存在时注册 `/api/auth/native-session`。
- Native session endpoint 只接受 loopback peer 与 `x-deve-native-session-secret`，secret 成功使用后立即消耗。
- Endpoint 成功后签发现有 JWT token，并返回 HttpOnly、SameSite=Strict、Path=/ 的 loopback cookie。
- Desktop bootstrap 先请求 native session cookie，再用该 cookie 验证 `/api/auth/status`，验证成功后才生成 session-bound Web bootstrap。
- 缺少 native session bootstrap secret 时直接 fail-closed，不回退到匿名 localhost 或裸 `/api/auth/status` handoff。
- Tauri bootstrap 使用 `on_webview_ready` 安装 cookie；cookie 安装失败必须在 session-bound bootstrap 前 fail-closed。
- JS bootstrap source 仍只包含 `http_base`、`ws_base`、`node_role`、`session_bound` 等非敏感信息。
- `DesktopNativeSessionCookie` 解析并校验 `token`、`HttpOnly`、`SameSite=Strict` 与 loopback domain，`Debug` 不暴露 cookie value。

## Not Opened

- 没有启用 `AUTH_ALLOW_ANONYMOUS_LOCALHOST`、dev secret fallback 或匿名本地绕过。
- 没有把 token/secret 放入 URL、localStorage、JS-visible bootstrap、日志或 crash report。
- 没有打开 Android process runtime、native authority writes、Web Git writer、signing、store、physical-device readiness 或 server-backed Settings API。
- 普通 Web login cookie contract 未改变；native loopback cookie 只服务本地 HTTP WebView bridge。

## Validation

- `cargo fmt --check`
- `cargo test --locked -p deve_cli native_session -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging service_entrypoint -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging service_bootstrap -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging tauri_bootstrap -- --nocapture`
- `cargo check --locked -p deve_desktop --features native-packaging`
- `cargo check --locked -p deve_cli`
- `cargo clippy --locked -p deve_desktop --all-targets --features native-packaging -- -D warnings`
- `cargo clippy --locked -p deve_cli --all-targets -- -D warnings`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/plan-coverage.sh`
- `git diff --check`

## Next

- 刷新 Desktop target-host evidence，确认 macOS/Windows Tauri package/startup 能通过 native session bridge。
- 继续保持 Mobile shell-only；Android process runtime 仍等待独立开启条件。

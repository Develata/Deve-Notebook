# Desktop Local Service Health Session Gate - 2026-05-17

本报告记录 Desktop local service health probe 与 session handoff gate。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/14_tech_stack.md`.
- Code scope: `apps/desktop/src/service_bootstrap.rs`, `apps/desktop/src/process_runtime.rs`, native gate scripts.
- Boundary: Desktop `native-packaging` only.

## Implemented

- 新增 `run_desktop_local_service_bootstrap`，按 start -> health probe -> endpoint bind -> session handoff -> Web bootstrap 顺序执行。
- health probe 读取 `/api/node/role`，只接受 loopback HTTP endpoint。
- session handoff 读取 `/api/auth/status`，只有 authenticated 才生成 session-bound bootstrap。
- probe 失败进入 `HealthProbeFailed`，handoff 失败进入 `SessionHandoffFailed`。
- bootstrap script 只在 endpoint healthy 且 session bound 后生成。
- runtime snapshot 与 shell recovery path 保持 authority-free。
- native gate scripts 反查 service bootstrap、node-role probe、auth-status handoff 与 Tauri entrypoint 未启动子进程。

## Not Opened

- Tauri startup 仍不自动启动 `deve_cli serve`。
- 未把 bootstrap script 注入 Tauri WebView。
- 未实现真实 native session secret 或 cookie bridge。
- 无 native authority write path。
- 无直接 ledger、vault、source-control、search-index、`.git` 或 `.notegit` 写入。
- 无 Android process runtime。
- 无 signing、store release、physical-device readiness、Web Git writer 或 server-backed Settings API。

## Validation

- `cargo fmt --check`
- `cargo test --locked -p deve_desktop --features native-packaging service_bootstrap -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging process_runtime -- --nocapture`
- `cargo check --locked -p deve_desktop --features native-packaging`
- `cargo clippy --locked -p deve_desktop --all-targets --features native-packaging -- -D warnings`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/plan-coverage.sh`
- `git diff --check`

## Next

Desktop local service Tauri bootstrap injection / session material bridge:

- 将 successful bootstrap 连接到 Tauri WebView initialization script 或等价 bootstrap 注入面。
- 实现不进 URL、localStorage、日志或 crash report 的 session material bridge。
- probe/handoff 失败时只注入 recovery bootstrap。
- 继续保持 writer-ready、repo scope 与 `scope_nonce` 由 server/core 决定。
- 继续关闭 Android process runtime 与 native authority writes。

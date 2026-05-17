# Desktop Local Service Entrypoint Plan Boundary - 2026-05-17

本报告记录 Desktop local service entrypoint 的最小边界实现。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/14_tech_stack.md`.
- Code scope: `apps/desktop/src/service_entrypoint.rs`, `apps/desktop/src/process_runtime.rs`, `apps/desktop/src/tauri_entry.rs`, native gate scripts.
- Boundary: Desktop `native-packaging` only.

## Implemented

- 新增 Desktop local service entrypoint planning module。
- 默认无 `DEVE_DESKTOP_LOCAL_SERVICE` 时保持 disabled。
- opt-in 只生成受控 spawn plan，不在 Tauri setup 中启动子进程。
- `deve_cli` executable 只从 Desktop executable sibling 解析，不接受任意 env path。
- spawn argv 固定为 `serve --native-loopback --port <port>`。
- spawn spec 绑定 loopback hints、data root、`config.toml`、`ledger/` 与 `vault/`。
- spawn spec 显式标记 health probe 与 session handoff 必须在 Web bootstrap 前完成。
- `DesktopCommandProcessLauncher` drop 时停止子进程，避免 runtime owner 释放后 orphan child。
- Desktop launcher 现在在 spawn 前重新校验完整 `NativeProcessSpawnSpec` contract。
- Desktop launcher 拒绝额外 serve argv、无效 port 与 argv port / bind hint 不一致。
- gate scripts 反查 Tauri entrypoint 不得提前 `app.manage` runtime 或调用 local-service start helper。

## Not Opened

- Tauri startup 仍不自动启动 `deve_cli serve`。
- 未执行真实 HTTP health probe。
- 未执行 session handoff。
- 未注入 Web bootstrap。
- 无 native authority write path。
- 无直接 ledger、vault、source-control、search-index、`.git` 或 `.notegit` 写入。
- 无 Android process runtime。
- 无 signing、store release、physical-device readiness、Web Git writer 或 server-backed Settings API。

## Validation

- `cargo fmt --check`
- `cargo test --locked -p deve_desktop --features native-packaging process_runtime -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging service_entrypoint -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging desktop_tauri -- --nocapture`
- `cargo check --locked -p deve_desktop --features native-packaging`
- `cargo clippy --locked -p deve_desktop --all-targets --features native-packaging -- -D warnings`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/check-native-packaging-gate.sh`

## Next

Desktop local service health probe / session handoff wiring:

- 启动 child 后必须先 probe `/api/node/role`。
- session handoff 必须在 endpoint healthy 后发生。
- Web bootstrap 只能在 endpoint healthy 与 session bound 后注入。
- probe/handoff 失败必须进入 recovery/offline path，不得显示半可写 UI。
- writer-ready、repo scope 与 `scope_nonce` 仍由 server/core 决定。

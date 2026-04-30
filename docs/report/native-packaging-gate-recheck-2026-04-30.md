# Native Packaging Gate Recheck - 2026-04-30

## 结论

Desktop/mobile native track 仍处于当前 no-Tauri skeleton 边界：`apps/desktop` 与 `apps/mobile` 只验收 endpoint/session/bootstrap/readiness/recovery contract。真实 Tauri/Tauri Mobile packaging、embedded child-process runtime、菜单/托盘/安装包、移动权限桥接与 store package 仍保持 post-gate future。

## 已确认

- `apps/desktop` 与 `apps/mobile` 默认构建只包含 native shell skeleton，不依赖 `tauri` 或 `tauri-build`。
- `native-packaging` feature 只暴露 planned scaffold，用于记录后续 dependency batch 与 forbidden authorities，不代表 packaging gate 已打开。
- `CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY` 仍为 `DeferredUntilRuntimeBatch`，且 `real_tauri_dependencies_allowed = false`。
- `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY` 仍为 `DeferredUntilPackagingGate`，且 `child_process_runtime_enabled = false`。
- desktop/mobile source tree 中未出现 `std::process`、`Command::new`、`tokio::process` 或 native skeleton `.spawn()` runtime。

## 本批次同步

- `scripts/check-native-track-boundary.sh` 新增 no process runtime leak 扫描。
- 同一脚本现在同时固定 desktop/mobile shell tests 对 `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY` 的覆盖，以及 core process policy 的 deferred/no-runtime 字段。
- `docs/report/next-tasks.md` 关闭 Native packaging gate recheck，并把下一步转为 post-queue plan/code drift rescan。

## 验证

- `scripts/check-native-track-boundary.sh`
- `cargo test -p deve_desktop`
- `cargo test -p deve_mobile`
- `git diff --check`

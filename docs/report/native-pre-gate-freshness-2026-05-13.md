# Native Pre-Gate Freshness

日期：2026-05-13

## 范围

- 计划锚点：`docs/plan/08_ui_design_02_desktop.md`、`docs/plan/08_ui_design_03_mobile.md`、`docs/plan/14_tech_stack.md#native-packaging-dependency-gate`
- 代码范围：`apps/desktop/`、`apps/mobile/`、`crates/core/src/native_adapter/`

## 结论

- Desktop / Mobile 当前仍处于 no-packaging skeleton 与 native adapter contract 阶段。
- `native-packaging` gate 仍关闭；真实 `tauri` / `tauri-build` dependency 未进入 workspace。
- Desktop shell / recovery / supervisor tests 通过。
- Mobile shell / recovery / supervisor tests 通过。
- Core native adapter endpoint/session/write-readiness/supervisor/process policy tests 通过。
- 本批不打开 Tauri/native packaging gate，不引入 native runtime dependency。

## 验证

- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/check-mobile-baseline.sh`
- `bash scripts/check-ui-desktop-baseline.sh`
- `cargo test -p deve_core native_adapter -- --nocapture`
- `cargo test -p deve_desktop -- --nocapture`
- `cargo test -p deve_mobile -- --nocapture`

## 测试计数

- `deve_core native_adapter`: 30 passed。
- `deve_desktop`: 21 passed。
- `deve_mobile`: 19 passed。
- `native-packaging` scaffold gate: desktop 3 passed, mobile 3 passed。

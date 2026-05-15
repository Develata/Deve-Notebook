# Desktop Package Startup Smoke

本报告记录 Desktop target-host package startup smoke 的本地实现批次。`docs/plan/` 仍为唯一权威；本批未修改 plan，未打开 child-process runtime，未授予 native authority write path。

## Scope

- 新增 packaged binary startup probe：`DEVE_DESKTOP_STARTUP_SMOKE=1`。
- 新增 `scripts/check-desktop-package-startup-smoke.sh`。
- 将 startup smoke 接入 manual `native-target-host.yml`，由 `run_desktop_startup_smoke=true` 显式开启。
- `scripts/check-desktop-platform-package-build.sh` 必须把 `native-packaging` feature 传给 `cargo tauri build`，否则 bundle 会产出默认 no-Tauri binary。
- 更新 release acceptance、runbook、release baseline 与 dispatch helper。

## Boundary

- Startup probe 只验证 Desktop binary 能启动并报告 shell-only runtime surface。
- Startup probe 在打开 GUI window、backend process、ledger、vault、source-control、search、Git、`.notegit` authority path 之前退出。
- 非 required 本地模式遇到旧的 no-Tauri release binary 时只诊断 skip；required target-host 模式 fail-closed。
- Startup probe 默认 20 秒超时，避免错误 binary 进入 GUI event loop 后卡住 target-host CI。
- Installer install/uninstall smoke 仍未实现，必须作为后续独立批次处理。

## Verification

- `cargo test --locked -p deve_desktop --features native-packaging desktop_tauri_startup_smoke -- --nocapture`
- `cargo check --locked -p deve_desktop --features native-packaging`
- `DEVE_DESKTOP_STARTUP_SMOKE=1 cargo run --locked -p deve_desktop --features native-packaging --bin deve_desktop`
- `scripts/check-desktop-package-startup-smoke.sh`
- `cargo build --locked -p deve_desktop --features native-packaging --release`
- `DEVE_DESKTOP_STARTUP_SMOKE_REQUIRED=1 scripts/check-desktop-package-startup-smoke.sh`
- `bash -n scripts/check-desktop-platform-package-build.sh`
- `DEVE_NATIVE_TARGET_HOST_TARGET=desktop-macos DEVE_NATIVE_TARGET_HOST_REQUIRED_PREFLIGHT=true DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_PACKAGE_BUILD=true DEVE_NATIVE_TARGET_HOST_RUN_DESKTOP_STARTUP_SMOKE=true scripts/dispatch-native-target-host-workflow.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `scripts/check-release-baseline.sh`
- `scripts/check-native-process-adapter-gate.sh`
- `scripts/check-desktop-package-preflight.sh`
- `scripts/plan-coverage.sh`
- `cargo fmt --check`

## Target-host Evidence

- macOS run `25914679737`: `desktop_preflight=success`, `process_gate=success`, `package_build=success`, `startup_smoke=success`.
- Windows run `25914681934`: `desktop_preflight=success`, `process_gate=success`, `package_build=success`, `startup_smoke=success`.
- macOS evidence artifact `deve-native-target-host-evidence-macos` validated with `scripts/collect-native-target-host-evidence.sh`.
- Windows evidence artifact `deve-native-target-host-evidence-windows` validated with `scripts/collect-native-target-host-evidence.sh`.
- Package artifacts were uploaded as `deve-desktop-macos-packages` and `deve-desktop-windows-packages`.

## Next

1. Add a separate installer install/uninstall smoke for macOS `.dmg/.app` and Windows MSI/NSIS.
2. Keep Mobile iOS target-host package execution as an independent pending target.
3. Keep process runtime gate closed until installer and iOS evidence are both closed.

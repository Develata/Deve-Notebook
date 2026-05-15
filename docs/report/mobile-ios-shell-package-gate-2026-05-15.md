# Mobile iOS Shell Package Gate

本报告记录 iOS shell-only package execution gate 的打开批次。`docs/plan/` 仍为唯一权威；本批只新增 iOS WebView 壳层 package gate，不打开 child-process runtime、native authority write path 或 release-ready 声明。

## Scope

- 新增 `08_ui_design_03_mobile#mobile-ios-shell-package-execution-gate`。
- 新增 `scripts/check-mobile-ios-shell-package-build.sh`。
- 将 `run_mobile_ios_package_build` 接入 manual `native-target-host.yml`。
- Mobile packaging contract 同时声明 Android/iOS shell-only package gate。

## Boundary

- iOS package build 只能在 macOS target host、`apps/mobile/native-packaging` feature 与显式 script 下运行。
- iOS CI/default package target 是 `aarch64-sim`，避免无签名 runner 触发 device signing。
- signed device IPA build 必须作为后续 signing gate 处理。
- 本地 Linux/WSL 默认只做 diagnostic；required 模式必须 fail-closed。
- iOS package execution 不运行 device/simulator install 或 startup smoke。
- iOS package execution 不启动、持有或重启后端子进程。
- iOS package execution 不写 ledger、vault、source-control、search index、Git 或 `.notegit` authority。

## Verification

- `cargo test --locked -p deve_mobile --features native-packaging packaging -- --nocapture`
- `scripts/check-mobile-ios-shell-package-build.sh`
- `DEVE_MOBILE_IOS_PACKAGE_BUILD_REQUIRED=1 scripts/check-mobile-ios-shell-package-build.sh` exits non-zero on Linux/WSL.

## Target-host Evidence

- GitHub run: `25917428903`
- Workflow: `Native Target Host`
- Target: `mobile-ios`
- Head: `a493d0bb`
- Host: macOS 15.7.4 arm64, Xcode 16.4
- Toolchain: Rust 1.92.0, Tauri CLI 2.11.1, Node.js 24.15.0
- Evidence artifact: `deve-native-target-host-evidence-ios`
- Package artifact: `deve-mobile-ios-packages`

Results:

- `mobile_ios_preflight=success`
- `process_gate=success`
- `package_build=success`
- iOS simulator package output includes `apps/mobile/gen/apple/build/arm64-sim/Deve Notebook.app`.
- Process runtime remains closed.
- Native authority writes remain closed.
- iOS install smoke and startup smoke were not requested by this package execution gate.

Corrections made during target-host validation:

- Run `25916157565` failed on signed device target `aarch64`; default target was changed to unsigned simulator `aarch64-sim`.
- Run `25916796983` built the simulator app, then failed because the native boundary scan inspected generated iOS artifacts; the scan now excludes generated package output and validates only hand-written source.

## Next

1. Keep iOS device signing as a separate future signing gate.
2. Keep iOS install/startup smoke as a separate simulator/device runtime gate.
3. Do not reopen child-process runtime or native authority writes from package build evidence alone.

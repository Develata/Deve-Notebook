# Native Target-host Tool Install Speedup

本报告记录 Native Target Host workflow 的工具安装加速批次。`docs/plan/` 仍为唯一权威；本批只调整 CI 工具安装路径，不改变 Desktop/Mobile runtime 权限边界。

## Scope

- 新增 `scripts/install-native-target-host-tools.sh`。
- Trunk 使用 `trunk-rs/trunk` 官方 release binary，版本固定为 `0.21.14`。
- Tauri CLI 使用 `tauri-apps/tauri` 官方 `tauri-cli-v2.11.1` release binary。
- `native-target-host.yml` 不再通过 `cargo install trunk` / `cargo install tauri-cli` 从源码编译工具。

## Boundary

- 该脚本只向 `target/native-tools/bin` 安装 CLI 工具，并通过 `GITHUB_PATH` 暴露给后续 workflow step。
- 版本校验必须通过：`trunk --version == trunk 0.21.14`，`cargo tauri --version == tauri-cli 2.11.1`。
- 不改变 package build、startup smoke、process runtime gate 或 native authority write path。

## Verification

- `bash -n scripts/install-native-target-host-tools.sh`
- `scripts/install-native-target-host-tools.sh`
- `DEVE_NATIVE_TOOL_BIN_DIR=target/native-tools-test/bin DEVE_NATIVE_INSTALL_TRUNK=1 DEVE_NATIVE_INSTALL_TAURI_CLI=1 scripts/install-native-target-host-tools.sh`
- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/native-target-host.yml")'`
- `scripts/check-release-baseline.sh`

## Next

Cancel the slow source-install target-host runs and dispatch macOS/Windows package build plus startup smoke again from the accelerated workflow commit.

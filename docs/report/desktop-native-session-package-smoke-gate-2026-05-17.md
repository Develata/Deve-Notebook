# Desktop Native Session Package Smoke Gate - 2026-05-17

本报告记录 Desktop native-session target-host evidence 的前置修正。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/09_auth.md`, `docs/plan/15_release.md`.
- Code scope: Desktop native-packaging startup smoke、Tauri package workflow、native target-host scripts。
- Boundary: Desktop `native-packaging` only。

## Implemented

- Desktop local-service spawn plan 为 child process 生成每次启动独立的 `AUTH_SECRET` 与 `AUTH_PASS`，避免 native-loopback server 依赖硬编码 dev defaults。
- `AUTH_SECRET` / `AUTH_PASS` 只进入受控 child-process env allowlist；`AUTH_ALLOW_ANONYMOUS_LOCALHOST` 不进入 Desktop spawn plan。
- Desktop required package build 先构建 `deve_cli`，再通过 Tauri `externalBin` transient config 注入 `binaries/deve_cli` sidecar；默认 `tauri.conf.json` 不加入 sidecar，避免普通 native-packaging tests 依赖本地 release binary。
- 新增 `scripts/check-desktop-native-session-package-smoke.sh`：显式 required 模式下运行 packaged Desktop binary，设置 `DEVE_DESKTOP_NATIVE_SESSION_SMOKE=1` 与 `DEVE_DESKTOP_LOCAL_SERVICE=1`，验证 bundled sibling `deve_cli` 能完成 native session cookie handoff。
- `native-target-host.yml` 在 Desktop macOS/Windows package startup smoke 后运行 native-session package smoke，并把 `native_session_smoke` 写入 evidence command results。

## Not Opened

- 没有打开 Android process runtime、native authority writes、signing、store、physical-device readiness、Web Git writer 或 server-backed Settings API。
- 没有启用 anonymous localhost bypass 或 hardcoded dev secret fallback。
- 没有把 sidecar 配置写入默认 `tauri.conf.json`；只在 required package build 的 transient config 中启用。

## Validation

- `cargo fmt --check`
- `cargo test --locked -p deve_desktop --features native-packaging service_entrypoint -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging service_bootstrap -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging desktop_tauri -- --nocapture`
- `cargo test --locked -p deve_desktop --features native-packaging tauri_bootstrap -- --nocapture`
- `cargo test --locked -p deve_cli native_session -- --nocapture`
- `cargo clippy --locked -p deve_desktop --all-targets --features native-packaging -- -D warnings`
- `cargo clippy --locked -p deve_cli --all-targets -- -D warnings`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/check-release-baseline.sh`
- `scripts/check-desktop-platform-package-build.sh`
- `scripts/check-desktop-native-session-package-smoke.sh`
- `git diff --check`

## Local Note

Linux required native-session package smoke is target-host dependent and failed on this WSL host before `main` due missing AppIndicator dynamic library. Non-required mode reports diagnostic skip and passes. macOS/Windows target-host workflow remains the authority for this evidence.

## Next

Push this gate, run `native-target-host.yml` for Desktop macOS and Desktop Windows with package build and startup smoke enabled, then collect and validate `deve-native-target-host-evidence-macos` / `deve-native-target-host-evidence-windows`.

# Apple Target-host Evidence Refresh - 2026-05-20

## Scope

本 session 仅覆盖 Apple target-host evidence refresh 范围：
- macOS Desktop package/startup/installer/native-session evidence
- iOS shell package/install/startup evidence

保持关闭（未打开）：signing、store、physical-device readiness、native authority writes、Mobile process runtime、Android process runtime、Web Git writer、server-backed Settings API。

未修改 `docs/plan/`。

## Baseline / Environment

- 当前 `HEAD`: `610b291 Record cloud full regression after local target-host smoke`
- `git pull origin main`: 失败（当前环境无可访问 `origin`）
- 用户指定基线 `1f100308`：当前仓库对象中不存在该 commit（`git merge-base --is-ancestor 1f100308 HEAD` 返回 invalid object）
- Host: Linux (`x86_64`), 非 Apple-capable target host
- `rustc`: `1.89.0`
- `cargo`: `1.89.0`

## Inputs Reviewed

- `docs/report/next-tasks.md`
- `docs/report/windows-desktop-smoke-2026-05-20.md`（文件不存在；仓库仅有 2026-05-18 / 2026-05-19 版本）
- `docs/report/post-regression-selection-after-windows-desktop-smoke-2026-05-20.md`（文件不存在）
- `docs/plan/08_ui_design_02_desktop.md`（按“Desktop native boundary / native-packaging / process adapter gate”关键词检索，未命中完全同名短语，改由相关 guard 脚本行为核对）

## Commands Run

1. `git pull origin main`
2. `git log -1 --oneline`
3. `git merge-base --is-ancestor 1f100308 HEAD`
4. `bash scripts/check-desktop-target-host-preflight.sh`
5. `bash scripts/check-desktop-platform-package-build.sh`
6. `bash scripts/check-desktop-package-startup-smoke.sh`
7. `bash scripts/check-desktop-native-session-package-smoke.sh`
8. `bash scripts/check-desktop-installer-smoke.sh`
9. `bash scripts/check-mobile-platform-package-preflight.sh`
10. `bash scripts/check-mobile-ios-shell-package-build.sh`
11. `bash scripts/check-mobile-ios-install-startup-smoke.sh`
12. `bash scripts/check-native-process-adapter-gate.sh`
13. `bash scripts/check-native-packaging-gate.sh`
14. `bash scripts/check-native-target-host-evidence.sh`

## Results

### Passed / Diagnostic pass

- `check-desktop-package-startup-smoke.sh`：diagnostic skip + `ok`（未启用 required 且缺 release binary）
- `check-desktop-native-session-package-smoke.sh`：diagnostic skip + `ok`（未启用 required 且缺 release binary/sidecar）
- `check-desktop-installer-smoke.sh`：Linux host 上按脚本语义 skip + `ok`
- `check-mobile-ios-install-startup-smoke.sh`：未启用 required 时 diagnostic skip + `ok`

### Failed

- `check-desktop-target-host-preflight.sh` / `check-desktop-platform-package-build.sh` 路径中：在 Linux 环境编译 `native-packaging` 触发 `glib-2.0` 缺失（`glib-sys` build script failed）
- `check-mobile-platform-package-preflight.sh` 与 `check-mobile-ios-shell-package-build.sh`：同样在 `native-packaging` 依赖检查阶段因 `glib-2.0` 缺失失败

### Skipped / Not runnable in current host

- macOS Desktop installer required smoke（需 macOS/Windows host）
- iOS simulator install/startup required smoke（需 Apple-capable host + required env）
- `check-native-target-host-evidence.sh`：本轮未生成新的 Apple target-host artifact 输入，需在 Apple target host workflow/artifact 上执行

## Key Logs

- `fatal: 'origin' does not appear to be a git repository`
- `fatal: Not a valid object name 1f100308`
- `desktop-installer-smoke-check: host_os=Linux`
- `desktop-installer-smoke-check: skip; installer smoke requires macOS or Windows target host`
- `mobile-ios-install-startup-smoke-check: install/startup not executed; set DEVE_MOBILE_IOS_INSTALL_STARTUP_SMOKE_REQUIRED=1 on a macOS simulator host`
- `The system library glib-2.0 required by crate glib-sys was not found`

## Boundary Status

- 本次未开启 signing/store/physical-device/native authority/process runtime/Web Git writer/server-backed Settings API。
- 未修改 `docs/plan/`。

## Decision

当前会话所在 Linux 云端不是 Apple-capable target host，且缺 `glib-2.0` native dependency，无法完成本次 Apple evidence refresh 的 required package/install/startup closure。应在 macOS target host（含 Xcode/simulator toolchain）上复跑上述 Apple-specific required gates 并再执行 evidence validator 收敛。

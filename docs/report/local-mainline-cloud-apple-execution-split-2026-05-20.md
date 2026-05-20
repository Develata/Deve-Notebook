# Local Mainline / Cloud Apple Execution Split - 2026-05-20

本报告记录后续多端执行规则调整。`docs/plan/` 未修改。

## Decision

- 主线开发、主线 full regression、Windows Desktop target-host smoke、Android Studio / emulator target-host smoke，默认在本机 Windows/WSL2 执行。
- Codex Cloud 不再承担主线 full regression 的默认执行职责。
- Codex Cloud 只在本机缺失 Apple 生态能力时启用，范围限定为 macOS / iOS target-host evidence，例如 macOS `.app/.dmg`、iOS simulator install/startup、Xcode/toolchain preflight。
- 跨端共享状态只通过 git commit、`docs/report/next-tasks.md` 与新增 `docs/report/*.md` report；不依赖聊天记忆。

## Rationale

- 本机已经具备 Rust、Node、Tauri、Windows installer、Android Studio、Android SDK/NDK 与 Android emulator，适合承接主线构建、测试、clippy、Web build、Windows Desktop smoke 与 Android package/emulator smoke。
- 云端 Codex 的 remote 可达性、长 Cargo 构建稳定性与 reasoning effort 控制不如本机可控；本次云端 full regression 尝试只确认到 `cargo fmt --check`，未收敛完整 gate。
- 本机缺口主要是 Apple target-host 能力；云端更适合作为 macOS/iOS evidence 补足入口。

## Updated Queue Shape

1. Local Full Regression Gate Refresh After Local Target-host Smoke Closure：在本机跑完整 full regression 并更新最终通过/失败/跳过矩阵。
2. Post-regression Work Selection：基于本机 full regression、`docs/plan/`、features、acceptance cases、guard scripts 与 Desktop/Android/iOS evidence 选择下一批小目标。
3. Cloud Apple Target-host Evidence Refresh：只有当 selection 需要 macOS / iOS evidence 时启动；不用于常规 Linux/Windows/Android/Web full regression。

## Boundaries Kept Closed

- signing
- store readiness
- physical-device readiness
- native authority writes
- Mobile process runtime
- Android process runtime
- Web Git writer
- server-backed Settings API

# Global Shortcut Parity

日期：2026-05-16

## 结论

- 已补齐 `Ctrl/Cmd+L`、`Ctrl/Cmd+Shift+O`、`Ctrl/Cmd+B`。
- 快捷键决策已收敛为纯函数 `plan_global_shortcut`，事件 handler 只负责阻止默认行为并执行已规划 action。
- Outline 使用 `OutlineControl` 共享布局状态，避免 Editor 内部重复创建独立 outline preference signal。
- Sidebar 使用 `SidebarControl` 共享布局控制面，Desktop sidebar 条件渲染，不再通过宽度 hack 模拟隐藏。
- Command Palette 的 `Toggle Sidebar` 已接到同一个 `SidebarControl`，与 `docs/features/12_commands.md` 的用户面描述一致。
- 未修改 `docs/plan/`。

## 验证

- `cargo test -p deve_web global_shortcut -- --nocapture`
- `cargo test -p deve_web static_commands_include_sidebar_toggle -- --nocapture`
- `bash scripts/check-cli-settings-baseline.sh`
- `cargo check -p deve_web --target wasm32-unknown-unknown`
- `cargo fmt --check`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `git diff --check`

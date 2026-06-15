# 12_commands.md - 命令体验篇

本章描述 CLI、Command Palette 与统一控制入口的用户体验。

## 功能目标

- 用户应能通过统一命令入口触达核心能力。
- 控件与命令应保持同一语义，而不是各走一套逻辑。

## Operation 示例

- 原子操作示例见 `docs/features/operations/ui_command_palette.md`。
- 该示例将 command palette 拆为打开、输入查询、结果导航、执行命令、关闭五个 user operations。
- Quick Open 与打开文档流示例见 `docs/features/operations/repo_open_doc.md`。
- CLI 控制面流示例见 [`operations/cli_control_commands.md`](./operations/cli_control_commands.md)。
- CLI 控制壳层细流见 [`operations/cli_parse_command.md`](./operations/cli_parse_command.md)、[`operations/cli_help_surface.md`](./operations/cli_help_surface.md)、[`operations/cli_empty_command_guidance.md`](./operations/cli_empty_command_guidance.md)、[`operations/cli_runtime_handoff.md`](./operations/cli_runtime_handoff.md)。
- 共享命令入口路由层见 [`operations/command_surface_mode_routing.md`](./operations/command_surface_mode_routing.md) 与 [`operations/command_surface_action_routing.md`](./operations/command_surface_action_routing.md)。
- SearchBox 文件操作命令壳层见 [`operations/repo_file_op_shell_routing.md`](./operations/repo_file_op_shell_routing.md)。
- 跨命令面板与设置的语言切换共享链见 [`operations/locale_surface_switch.md`](./operations/locale_surface_switch.md)。
- CLI 细粒度命令族见 [`operations/cli_projection_workspace_indexing.md`](./operations/cli_projection_workspace_indexing.md)、[`operations/cli_server_runtime.md`](./operations/cli_server_runtime.md)、[`operations/cli_export_inspect.md`](./operations/cli_export_inspect.md)、[`operations/cli_repair_admin.md`](./operations/cli_repair_admin.md)。

## 功能项

### 1. Command Palette

- 用户可以打开命令面板，搜索并执行主要命令。
- 命令面板应覆盖核心导航与工作流入口。

### 2. Quick Open / Branch Switch / Sidebar Toggle

- 关键工作流应有直接命令入口或快捷键。
- 这些入口触发的结果应与对应控件行为一致。

### 3. CLI

- CLI 命令是系统控制面的正式组成部分。
- 它不只是调试工具，也服务于多端共享的 application/control 路径。

### 4. Git Mirror Repair Command Boundary

- `Git: Repair Mirror` 当前是 Command Palette 可发现入口，但只打开 Source Control 的 CLI-only notice。
- 该 notice 指向 `deve_cli git status --repo <repo>` 的 `repair_action[...]` /
  `repair_guidance[...]`，以及 `deve_cli git export --repo <repo> --retry-out-of-sync`。
- 下一阶段若加入可点击 repair UI，Command Palette 只能进入 repair review flow，不能绕过 Source Control gate 直接写 Git。
- 可执行 repair 必须要求 manual confirmation；confirmation 前只能展示诊断、subject、next step 与 copyable retry command。

### 5. Git Mirror Import/Export/Push Command Chain

- 当前 resolved import 发布链路必须通过 CLI 显式执行：`deve_cli git import --apply` 只写 pending/import；Source Control resolved stage/commit 生成 Deve commit；`deve_cli git export` 建立 Git mirror mapping；`deve_cli git push` 发布已映射 Git HEAD。
- `deve_cli git push` 必须 fail-closed 于未导出的 queued/out_of_sync mirror record、dirty Git worktree、dirty Deve Source Control、未映射 Git HEAD 或 remote/branch 配置错误。
- Web Command Palette 只能显示 Git import / push / repair 的 CLI-only notice，不得直接触发 Git writer；notice metadata 中的 `source_control.git_bridge` 必须跟随当前 node role / session mode 更新。

## 非目标

- 当前阶段不允许只有显示层按钮能做、命令层做不到的核心能力。
- 当前阶段不要求把所有未来扩展命令都默认暴露给用户。
- 当前阶段不允许 Command Palette 触发后台自动 Git repair。

## Chrome MCP 验收实例

### CMD-FEAT-01: 命令面板触达核心能力

前置条件：

- 按 `docs/dev-runbook.md` 启动本地 Web shell。

步骤：

1. 打开 Command Palette。
2. 搜索并执行 `Open Settings`、`Toggle Sidebar`、`Switch Branch` 或等价核心命令。
3. 观察页面变化。

期望结果：

- 命令可被搜索到并执行。
- 执行结果与对应控件语义一致。

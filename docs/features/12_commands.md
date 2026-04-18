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
- CLI 细粒度命令族见 [`operations/cli_vault_indexing.md`](./operations/cli_vault_indexing.md)、[`operations/cli_server_runtime.md`](./operations/cli_server_runtime.md)、[`operations/cli_export_inspect.md`](./operations/cli_export_inspect.md)、[`operations/cli_repair_admin.md`](./operations/cli_repair_admin.md)。

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

## 非目标

- 当前阶段不允许只有显示层按钮能做、命令层做不到的核心能力。
- 当前阶段不要求把所有未来扩展命令都默认暴露给用户。

## Chrome MCP 验收实例

### CMD-FEAT-01: 命令面板触达核心能力

前置条件：

- 打开应用主界面。

步骤：

1. 打开 Command Palette。
2. 搜索并执行 `Open Settings`、`Toggle Sidebar`、`Switch Branch` 或等价核心命令。
3. 观察页面变化。

期望结果：

- 命令可被搜索到并执行。
- 执行结果与对应控件语义一致。

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
- `deve watch` 是 standalone owned-watcher runtime：直接持有不可复制的 repo handle 集合；任一 worker terminal failure 都必须逆序显式 shutdown 全部 handle 并非零退出，不得依赖全局 registry 或 `Drop` 静默成功。每个 handle 的 shutdown 统一执行 producer stop/join、queued-hint discard、exact-root final reconcile 与至多一次 refresh，并保留 worker primary 与 cleanup diagnostics。
- `deve serve` 由host `WatcherSupervisor`隔离repo-local ingestion failure；zero-repo host以`NoScope`正常启动且watcher expected=0为healthy。只有typed host-fatal才清理并非零退出；存在repo但全部watcher Failed时仍提供readonly/diagnostic/Create能力。
- `deve repo remove --repo-id <uuid>`默认只执行backend Prepare并显示safe preview、preparation id与opaque token；
  `deve repo remove --repo-id <uuid> --apply --token <opaque>`才执行。server持有authority时两次调用都走
  authenticated loopback proxy，且在读取password前必须匹配ledger-local owner hint与server process incarnation；offline两次invocation绑定稳定authority-root/lock identity而不是短命process。
  输出按backend顺序列出deleted/preserved/warning/blocker，不打印path、manifest、digest或raw cleanup detail。
  normal Prepare/Execute现已可用；`deve repo removal-repair --request-id <uuid>`仍是首发阻塞目标，必须在
  owner-specific typed repair API完成后默认dry-run，实际清理再提供`--apply --token`，当前不得用普通
  Execute或pathname脚本替代。
- CLI 输出只消费 typed failure/mount outcome；不得按 backend 错误字符串决定 repo 隔离、host shutdown 或恢复动作。

### 4. NoteGit / Git Main Mirror Repair Command Boundary

- `ngit:repair` 当前是 Command Palette 可发现入口，但只打开 Source Control 的 read-only notice/review。
- 该 notice 指向 `deve_cli ngit status --repo <repo>` 的 `repair_action[...]` /
  `repair_guidance[...]`，以及 `deve_cli ngit export --repo <repo> --retry-out-of-sync`。
- 下一阶段若加入可点击 repair UI，Command Palette 只能进入 repair review flow，不能绕过 Source Control gate 直接写 Git。
- 可执行 repair 必须要求 manual confirmation；confirmation 前只能展示诊断、subject、next step 与 copyable retry command。

### 5. NoteGit Mirror Import/Export/Push Command Chain

- 当前 resolved import 发布链路必须通过 runtime 显式执行：`deve_cli ngit import --apply` 只写 pending/import；External Changes / Apply to Ledger / Source Control commit 生成 NoteGit/ngit facts 与 commit anchor；`deve_cli ngit export` 建立 Git main mirror mapping；`deve_cli ngit push` 发布已映射 Git HEAD。
- CLI 等价链路为 `deve_cli sc stage --all` → `deve_cli sc apply` →
  `deve_cli sc commit --message <message>`；普通 commit 不消费 External Changes staging。
- `deve_cli ngit push` 必须 fail-closed 于未导出的 queued/out_of_sync mirror record、dirty Git worktree、dirty NoteGit Source Control、未映射 Git HEAD 或 remote/branch 配置错误。
- Web Command Palette 只能显示 ngit import / push / repair 的 backend/runtime intent 或 read-only notice，不得直接触发 Git writer；不再读取 `source_control.git_bridge` mode。

### 6. Remote Projection Push and Remote Import Commands

Remote Projection Push 只有在当前 Web session、repo scope、非零 scope nonce 与 handshake 均可构造精确
typed intent 时才显示为 available；否则显示本地化 unavailable reason。前端不据此推导 provider、Mounted
或 writer authority，最终准入仍由 backend typed gate 决定。

- Command Palette 的正式 CommandId 固定为：
  - `remote_projection.webdav.push`
  - `remote_projection.s3.push`
  - `remote_import.webdav.prepare`
  - `remote_import.s3.prepare`
  - `remote_import.open`
  - `remote_import.refresh`
  - `remote_import.apply`
  - `remote_import.discard`
- List/Show/Page/Diff 是 Remote Import view 内部 typed request，不增加全局 CommandId。
- push 只上传 Markdown Projection Workspace files；Remote Import Prepare 只创建 immutable captured
  session，不覆盖 Workspace、不触发 External Changes、不写 Ledger。
- Remote Import Open/Refresh/Apply/Discard 只打开独立 view 或发送 whole-session typed intent；Command
  Palette 不直接访问 provider、manifest/blob、Ledger writer 或 cleanup runtime。
- Web Command Palette 不接收 locator、endpoint URL、host path、digest 或 credential material；
  backend 从当前 repo 与 backend-defined profile handle 完成 admission。
- CLI 对应入口为 `projection-remote <webdav|s3> push` 与
  `remote-import prepare|list|show|diff|refresh|apply|discard|repair`；repair 默认 dry-run，只有
  `--apply` 才清理已证明 artifact。
- 当前 Web 只注册两个稳定 Push CommandId，并发送 exact repo/branch/scope 的 typed backend intent；
  旧四个 id、workspace overwrite 与 External Changes bridge 已删除，不保留 alias。

## 非目标

- 当前阶段不允许只有显示层按钮能做、命令层做不到的核心能力。
- 当前阶段不要求把所有未来扩展命令都默认暴露给用户。
- 当前阶段不允许 Command Palette 触发后台自动 Git repair。
- 当前阶段不允许 Command Palette 直接访问 WebDAV/S3 provider。
- 当前阶段不允许 Remote Import checkbox、逐文件 Apply、前端 blocker 推理或 raw detail parsing。

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

# 14_commands.md - 命令篇 (Commands)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Planned / Optional`
- `Counterpart Feature`: `docs/features/12_commands.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/11_commands_settings.md`
- `Primary Code Areas`: `apps/cli/src/commands/`, `apps/web/src/components/command_palette/`

本章汇总系统涉及的所有 CLI 命令与 Command Palette 指令。

权威状态以 `docs/plan/deve-note plan.md` 为准：本章是规划/扩展契约，不能反向覆盖
`01/02/04/05/06/07/09/11` 的硬约束章节。

命令面分为三类：

*   **Baseline CLI Contract**：与 core authority、配置读取、诊断和修复直接相关的 CLI surface；启用时 **MUST** fail-closed 并返回结构化错误。
*   **Optional Bridge Contract**：Git mirror、AI backend、Trusted CLI 等外围能力；启用时 **MUST** 服从对应章节的 authority 边界。
*   **Future UI Surface**：Command Palette 中未绑定后端能力的入口 **MAY** 以 disabled/unavailable 状态出现，但 **MUST NOT** 伪装成可执行能力。

## 1. CLI Commands {#cli-commands}

*   **Baseline CLI Contract**:
    *   `deve init --repo <name> --projection-base <path>`: 初始化 ledger 与首个本地 repo，并注册该 repo 的 Projection Locator；最终 workspace root 为 `<path>/<name>/`。
    *   `deve repo projection set --repo <selector> --base <path>`: 为本地 repo 创建或替换 projection base；必须停止 watcher、校验 locator、重建 projection，再恢复 repo runtime。
    *   `deve repo projection list`: 列出本机 host-local Projection Locator。
    *   `deve repo projection check --repo <selector>`: 只读校验 projection base 与计算出的 workspace root 是否存在、可 canonicalize、无冲突。
    *   `deve scan`: 扫描当前已绑定 workspace root 的 repo 并建立索引.
    *   `deve watch`: 监听已绑定 workspace root 的 repo 文件变更.
    *   `deve serve`: 启动 WebSocket 服务端.
    *   `deve dump`: 调试工具 (Dump Ops).
    *   `deve export`: 导出 Ledger 为 JSONL；Markdown 导出遇到 degraded projection 时必须要求显式 `--allow-degraded-projection`。
    *   `deve graph`: 输出当前 repo 的只读 `GraphProjection` JSON；默认要求健康 Structure Facts authority，显式 `--allow-degraded-projection` 才允许从 metadata fallback 导出。
    *   `deve verify-p2p`: P2P 逻辑验证.
    *   `deve seed`: 种子节点数据注入.
    *   `deve node-check`: 节点一致性检查，可选修复；`--projection` 执行只读 Structure Facts / projection authority 诊断。
    *   `deve recover`: 从 ledger 数据恢复 repo projection workspace 文件。
    *   `deve repair`: 修复已知本地损坏并可重建投影；当 Structure Facts authority 已损坏时必须输出诊断并 fail-closed。
    *   `deve config print`: 输出当前有效运行时配置。
    *   `deve config set <key> <value>`: 写入受支持的 `config.toml` 键。
*   **Optional Bridge Contract**:
    *   Git mirror 命令的 lifecycle、preflight、import/export/push blocker 与 repair 语义以 `07_diff_logic.md#git-mirror-lifecycle` 为唯一权威。
    *   `deve git status`: 只读检查 Git mirror readiness 与 queue/out-of-sync summary。
    *   `deve git mirror`: 显式执行 queued/out-of-sync Git mirror records。
    *   `deve git export`: 显式导出 Deve commits 到 Git mirror。
    *   `deve git import`: 只读规划外部 Git/worktree changes。
    *   `deve git import --apply`: 显式把安全 Git changes 写入 pending/import；不得直接生成 ledger facts。
    *   `deve git push`: 显式发布已映射 `.git` mirror HEAD。

## 2. Command Palette {#command-palette-shortcuts}

*   **Global Shortcuts**:
    *   `Cmd+Shift+P` / `Ctrl+Shift+P`: 呼出 Command Palette (指令导航)。
    *   `Cmd+P` / `Ctrl+P`: 呼出 Quick Open (文件跳转)。
    *   `Cmd+Shift+K` / `Ctrl+Shift+K`: 呼出 branch 切换。

*   **Source Control / Git-like Workflow**:
    *   `Source Control: Sync`: 同步 Deve repo-scoped changes.
    *   `Source Control: Commit`: 提交 staged changes 到 ledger-backed commit anchor.
    *   `Source Control: Push`: 推送 Deve source-control state；不得被解释为 Web 直接执行 Git mirror push；Git mirror publish 只由显式 `deve git push` surface 承担。
    *   `Git: Status`: 只读查看 `.git` mirror readiness、repo-local `.gitignore` 是否保护 `.notegit/`，以及 `GitMirrorQueued / Committed / OutOfSync` 队列状态。
    *   `Git: Mirror`: 显式执行 queued Git mirror commit；执行面 **MUST** 复用第 7 章 Git mirror preflight 与 out-of-sync 边界。
    *   `Git: Export Mirror`: 将 queued Deve projection commits 导出到 Git mirror，并建立 Deve commit 到 Git commit 的映射。
    *   `Git: Import Changes`: 将外部 Git/worktree 变化转成 pending/import，再进入 Deve stage/commit；该命令不得直接生成 ledger commit。
    *   `Git: Push Mirror`: 将已映射的 `.git` mirror HEAD 推送到远端；不得绕过 Deve authority。
    *   `Git: Repair Mirror`: 可展示 repair/retry 指引；任何 Git write **MUST** 经过显式确认，并 fail-closed 于第 7 章定义的 blocker。
    *   `Git:*` 文案 **MAY** 作为兼容 alias 出现，但不得被解释为 `.git/` 是 Deve runtime authority。

*   **P2P / Branch**:
    *   `P2P: Switch to Peer`: 切换到指定 Peer 的影子分支.
    *   `P2P: Establish Branch`: 从当前查看的 Peer 分支创建本地分支.
    *   `P2P: Merge Peer`: 将当前 Spectator Mode 查看的 Peer 分支合并入本地.
    *   `Repo: Set Projection Base`: 为本地 repo 绑定 projection base；执行面必须复用 CLI locator 校验合同。
    *   `Repo: Check Projection Workspace`: 只读检查 repo projection base 与 workspace root readiness。

*   **命令执行边界**:
    *   Command Palette 入口启用时 **MUST** 调用明确 backend contract；未启用时 **MUST** 显示 disabled/unavailable 状态。
    *   CLI-only notice 只能作为可发现性入口，**MUST NOT** 被解释为 Web 已能直接执行 Git import/push/repair。
    *   Git repair 的可点击 UI、完整 conflict UI 与后台自动 repair **MAY** 作为 future UI surface 另行设计；只读 notice 或 review surface **MUST NOT** 隐式升级为 Git writer。

*   **交互准则 (Command First)**:
    *   大多数功能必须通过命令面板触发，减少 UI 按钮密度。
    *   底部状态栏仅展示 AI 模式与基础统计。

*   **AI**:
    *   `AI: Open Chat`: 打开统一 AI 面板。
    *   `AI: Retry Last Request`: 重试上一条失败请求。
    *   `AI: Switch Backend`: 在 Native AI Chat 与 Trusted CLI Agent（仅在显式启用且满足 trusted 条件时）之间切换。
    *   `AI: Switch to PLAN Mode`: 将原生 AI 切换到只读规划模式。
    *   `AI: Switch to BUILD Mode`: 将原生 AI 切换到执行模式。

## 3. Chat Slash Commands

*   `/plan`: 切换到原生 `PLAN` 模式。
*   `/build`: 切换到原生 `BUILD` 模式。
*   `/agents`: 在原生 `PLAN ↔ BUILD` 之间顺序切换。

### Slash Command 语义

*   `PLAN`：
    - 禁止调用任何工具。
    - 只输出分析、计划、步骤与建议。
*   `BUILD`：
    - 允许直接修改当前 Markdown。
    - 允许调用受控的程序执行路径来完成 Markdown 修改。
    - 不等于 MCP、开放 Skills 或任意 shell；MCP 不属于当前命令体系。
*   `agents`：
    - 仅作用于原生 `PLAN / BUILD` 两种聊天模式。
    - 不负责切换 `native / trusted-cli` 后端。

### Backend 命令可见性

*   `AI: Switch Backend` 仅在以下条件都成立时才应可见或可用：
    - `ai.agent_bridge.enabled = true`
    - `ai.agent_bridge.trusted = true`
    - `AGENT_CLI_PATH` 已配置
*   条件不满足时，系统 **MUST** 保持 `native` 后端，并对用户给出明确说明，而不是静默尝试拉起 CLI。

## 4. Related Commands (本章相关命令)

*   无 (本章即为命令汇总)。

## 5. Related Configuration (本章相关配置)

*   无。

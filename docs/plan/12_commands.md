# 12_commands.md - 命令篇 (Commands)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Planned / Optional`
- `Counterpart Feature`: `docs/features/12_commands.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/11_commands_settings.md`
- `Primary Code Areas`: `apps/cli/src/commands/`, `apps/web/src/components/command_palette/`

本章汇总系统涉及的所有 CLI 命令与 Command Palette 指令。

当前权威状态以 `docs/plan/deve-note plan.md` 为准：本章是规划/扩展契约，不能反向覆盖
`01/02/04/05/06/07/09/11` 的 Current MUST。已实现命令应在本章保持可追踪；未实现命令
必须被视为 future work，不能作为当前验收阻塞项。

## 1. CLI Commands {#cli-commands}

*   `deve init`: 初始化 Vault.
*   `deve scan`: 扫描并建立索引.
*   `deve watch`: 监听文件变更.
*   `deve serve`: 启动 WebSocket 服务端.
*   `deve dump`: 调试工具 (Dump Ops).
*   `deve export`: 导出 Ledger 为 JSONL；Markdown 导出遇到 degraded projection 时必须要求显式 `--allow-degraded-projection`。
*   `deve verify-p2p`: P2P 逻辑验证.
*   `deve seed`: 种子节点数据注入.
*   `deve node-check`: 节点一致性检查，可选修复；`--projection` 执行只读 Structure Facts / projection authority 诊断。
*   `deve recover`: 从 ledger 数据恢复 vault 文件。
*   `deve repair`: 修复已知本地损坏并可重建投影；当 Structure Facts authority 已损坏时必须输出诊断并 fail-closed。
*   `deve config print`: 输出当前有效运行时配置。
*   `deve config set <key> <value>`: 写入受支持的 `config.toml` 键。

## 2. Command Palette {#command-palette-shortcuts}

*   **Global Shortcuts**:
    *   `Cmd+Shift+P` / `Ctrl+Shift+P`: 呼出 Command Palette (指令导航)。
    *   `Cmd+P` / `Ctrl+P`: 呼出 Quick Open (文件跳转)。
    *   `Cmd+Shift+K` / `Ctrl+Shift+K`: 呼出 branch 切换。

*   **Source Control / Git-like Workflow**:
    *   `Source Control: Sync`: 同步 Deve repo-scoped changes.
    *   `Source Control: Commit`: 提交 staged changes 到 ledger-backed commit anchor.
    *   `Source Control: Push`: 推送 Deve source-control state；若启用 Git mirror，可触发 mirror publish。
    *   `Git: Status`: 查看 `.git` mirror 是否存在、是否落后、是否 `GitMirrorOutOfSync`。
    *   `Git: Export Mirror`: 将当前 projection 导出到 Git mirror，并建立 Deve commit 到 Git commit 的映射。
    *   `Git: Import Changes`: 将外部 Git/worktree 变化转成 pending/import，再进入 Deve ledger commit。
    *   `Git: Push Mirror`: 将 Git mirror 推送到远端；不得绕过 Deve authority。
    *   `Git:*` 文案 MAY 作为兼容 alias 出现，但不得被解释为 `.git/` 是 Deve runtime authority。

*   **P2P / Branch**:
    *   `P2P: Switch to Peer`: 切换到指定 Peer 的影子分支.
    *   `P2P: Establish Branch`: 从当前查看的 Peer 分支创建本地分支.
    *   `P2P: Merge Peer`: 将当前 Spectator Mode 查看的 Peer 分支合并入本地.

*   **Current implemented subset**:
    *   Command Palette 当前覆盖 Open / Settings / Toggle Language / Switch Peer /
        Establish Branch / Merge Peer / Toggle AI Chat（条件可见）。
    *   Source Control Sync / Commit / Push、Git mirror commands 与 AI Retry / Backend /
        PLAN / BUILD 面板命令仍属于 Planned / Optional，除非后续验收用例绑定到具体实现。

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

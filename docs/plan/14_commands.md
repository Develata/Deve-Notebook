# 14_commands.md - 命令篇 (Commands)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Planned / Optional`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-04`
- `Counterpart Feature`: `docs/features/12_commands.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/11_commands_settings.md`
- `Primary Code Areas`: `apps/cli/src/commands/`, `apps/web/src/context_action/`, `apps/web/src/components/command_palette/`

本章汇总系统涉及的所有 CLI 命令与 Command Palette 指令。

权威状态以 `docs/plan/deve-note plan.md` 为准：本章是规划/扩展契约，不能反向覆盖
Current MUST 硬约束章节（`01_terminology`/`02_positioning`/`03_storage`/`04_repository`/`05_diff_logic`/`07_network`/`08_auth`/`13_i18n`）。

命令面分为三类：

*   **Baseline CLI Contract**：与 core authority、配置读取、诊断和修复直接相关的 CLI surface；启用时 **MUST** fail-closed 并返回结构化错误。
*   **Optional Bridge Contract**：Git main mirror、WebDAV/S3 projection transport、AI backend、Trusted CLI 等外围能力；启用时 **MUST** 服从对应章节的 authority 边界。
*   **Future UI Surface**：Command Palette 中未绑定后端能力的入口 **MAY** 以 disabled/unavailable 状态出现，但 **MUST NOT** 伪装成可执行能力。

## 1. CLI Commands {#cli-commands}

*   **Baseline CLI Contract**:
    *   `deve init --path <data-root> --repo <name> --projection-base <projection-base> [--repo-id <uuid>] [--repo-url <url>]`: 初始化 `<data-root>` 下的 config / ledger 与首个本地 repo，并注册该 repo 的 host-local Projection Locator；`--path` 选择 CLI 数据/配置根，不是 Projection Workspace authority；最终 workspace root 为 `<projection-base>/<safe_repo_name>--<repo_id>/`。`--repo-id` 只允许在创建新 repo 时写入；若既有 repo metadata 与传入 `RepoId` 不一致，必须 fail-closed。
    *   `deve repo projection set --repo <selector> --base <path>`: 为本地 repo 创建或替换 projection base；必须停止 watcher、校验 locator、重建 projection，再恢复 repo runtime。
    *   `deve repo projection list`: 列出本机 host-local Projection Locator。
    *   `deve repo projection check --repo <selector>`: 只读校验 projection base 与计算出的 workspace root 是否存在、可 canonicalize、无冲突。
    *   `deve repo projection drift --repo <selector> [--root <path>]`: 只读列出 ledger projection 与指定 workspace root 的 unexplained drift；不得写 ledger、workspace、pending 或 staged state。
    *   `deve scan`: 扫描当前已绑定 workspace root 的 repo 并建立索引.
    *   `deve watch`: 监听已绑定 workspace root 的 repo 文件变更.
    *   `deve serve`: 启动 WebSocket 服务端.
    *   `deve dump`: 调试工具 (Dump Ops).
    *   `deve export`: 导出 Ledger 为 JSONL；Markdown 导出遇到 degraded projection 时必须要求显式 `--allow-degraded-projection`。
    *   `deve graph`: 输出当前 repo 的只读 `GraphProjection` JSON；默认要求健康 Structure Facts authority，显式 `--allow-degraded-projection` 才允许从 metadata fallback 导出。
    *   `deve verify-p2p`: P2P 逻辑验证；默认保留本地 shadow 隔离模拟，显式 mesh smoke 入口必须通过 Docker/运行时脚本验证真实 server-to-server `/ws`。
    *   `deve seed`: 种子节点数据注入.
    *   `deve node-check`: 节点一致性检查，可选修复；`--projection` 执行只读 Structure Facts / projection authority 诊断。
    *   `deve recover`: 从 ledger 数据恢复 repo projection workspace 文件。
    *   `deve sc status --repo <selector>`: 只读输出 Deve Source Control staged / unstaged 状态；`deve sc-status` 可作为兼容别名保留。
    *   `deve sc stage --repo <selector> --all`: 将当前 repo 的 pending / working changes 显式移入 staged；执行面必须复用 `05_diff_logic` 的 target resolution 与 stage 边界。
    *   `deve sc commit --repo <selector> --message <message>`: 将 staged/confirmed changes 提交为 NoteGit/ngit ledger-backed commit anchor；不得执行 Git push，也不得直接写 Git index。
    *   `deve repair`: 修复已知本地损坏并可重建投影；当 Structure Facts authority 已损坏时必须输出诊断并 fail-closed。
    *   `deve config print`: 输出当前有效运行时配置。
    *   `deve config set <key> <value>`: 写入受支持的 `config.toml` 键。
*   **Optional Bridge Contract**:
    *   Git main mirror lifecycle、preflight、import/export/push blocker 与 repair 语义以 `05_diff_logic.md#git-mirror-lifecycle` 为唯一权威。
    *   Git main mirror 不再有 `source_control.git_bridge` 配置；NoteGit/ngit commit 成功后始终尝试排队 mirror record。
    *   `deve ngit status`: 只读检查 NoteGit authority、Git main mirror readiness 与 queue/out-of-sync summary。
    *   `deve ngit mirror`: 显式执行 queued/out-of-sync Git main mirror records。
    *   `deve ngit export`: 显式导出 NoteGit/ngit projection 终态到 Git main mirror。
    *   `deve ngit import`: 只读规划外部 Git/worktree changes。
    *   `deve ngit import --apply`: 显式把安全 Git changes 写入 pending/import；不得直接生成 ledger facts。
    *   `deve ngit push`: 显式发布已映射 `.git` main mirror HEAD。
    *   `deve projection-remote webdav push/pull` 与 `deve projection-remote s3 push/pull`: 通过 backend/core runtime 执行 Projection Backup / Remote Projection transport；push/pull 只传输 Markdown Projection Workspace files，pull 只覆盖 projection files 并进入 External Changes，不直接写 ledger。

## 2. Command Palette {#command-palette-shortcuts}

*   **Global Shortcuts**:
    *   `Cmd+Shift+P` / `Ctrl+Shift+P`: 呼出 Command Palette (指令导航)。
    *   `Cmd+P` / `Ctrl+P`: 呼出 Quick Open (文件跳转)。
    *   `Cmd+Shift+K` / `Ctrl+Shift+K`: 呼出 branch 切换。

*   **Source Control / NoteGit-like Workflow**:
    *   `Source Control: Sync`: 同步 Deve repo-scoped changes.
    *   `Source Control: Commit`: 提交 staged changes 到 ledger-backed commit anchor.
    *   `Source Control: Push`: 推送 Deve source-control state；不得被解释为 Web 直接执行 Git push；Git main mirror publish 只由显式 backend/runtime surface 承担。
    *   `ngit:status`: 只读查看 `.git` main mirror readiness、repo-local `.gitignore` 是否保护 `.notegit/`，以及 `GitMirrorQueued / Committed / OutOfSync` 队列状态。
    *   `ngit:mirror`: 显式执行 queued Git main mirror commit；执行面 **MUST** 复用 `05_diff_logic` 的 Git mirror preflight 与 out-of-sync 边界。
    *   `ngit:export`: 将 queued NoteGit/ngit projection 终态导出到 Git main mirror，并建立 NoteGit/ngit commit 到 Git commit 的映射。
    *   `ngit:import`: 将外部 Git/worktree 变化转成 pending/import，再进入 External Changes / Apply to Ledger / Source Control commit；该命令不得直接生成 ledger commit。
    *   `ngit:push`: 将已映射的 `.git` main mirror HEAD 推送到远端；不得绕过 NoteGit/ngit authority。
    *   `ngit:repair`: 可展示 repair/retry 指引；任何 Git write **MUST** 经过显式确认，并 fail-closed 于 `05_diff_logic` 定义的 blocker。
    *   `webdav:push` / `webdav:pull` / `s3:push` / `s3:pull`: 发送 remote projection transport intent；前端不得直接访问 WebDAV/S3 provider。
    *   Command Palette / Source Control UI 不再展示 `source_control.git_bridge` mode；Source Control header copy **MUST** 明确 `.notegit` / NoteGit/ngit 是 authority、Git main 只是终态 mirror；Web surface **MUST NOT** 直接执行 Git writer。
    *   若旧客户端仍发送 `ClientMessage::CommitAndPush`，服务端 **MUST** 先复用当前 Source Control write gate，然后返回结构化 unsupported blocker；不得创建重复 NoteGit/ngit commit、排队重复 Git mirror record 或执行 Git push。
    *   代理 / plugin-host 模式下，Command Palette 不得读取或展示 legacy bridge mode；delegated/readonly 状态必须来自 runtime typed state。
    *   `Git:*` 文案不再作为 v1 command surface 出现；需要 Git ecosystem mirror 诊断时使用 `ngit:*`。

*   **P2P / Branch**:
    *   `P2P: Switch to Peer`: 切换到指定 Peer 的影子分支.
    *   `P2P: Establish Branch`: 从当前查看的 Peer 分支创建本地分支.
    *   `P2P: Merge Peer`: 仅在 Local Branch 下可执行；用户显式选择只读 peer mirror / shadow source，并将结果合并写入本地 ledger。
    *   `P2P: Mesh Status`: 只读查看静态 peer connector 状态；不得执行 discovery 或自动 merge。
    *   `Repo: Set Projection Base`: 为本地 repo 绑定 projection base；执行面必须复用 CLI locator 校验合同。
    *   `Repo: Check Projection Workspace`: 只读检查 repo projection base 与 workspace root readiness。

*   **命令执行边界**:
    *   Command Palette 入口启用时 **MUST** 调用明确 backend contract；未启用时 **MUST** 显示 disabled/unavailable 状态。
    *   ngit / Git-main-mirror notice 只能作为可发现性入口或只读 review 入口，**MUST NOT** 被解释为 Web 已能直接执行 Git import/push/repair。
    *   Git mirror repair 的可点击 UI、完整 conflict UI 与后台自动 repair **MAY** 作为 future UI surface 另行设计；只读 notice 或 review surface **MUST NOT** 隐式升级为 Git writer。
    *   每个 Command Palette entry **MUST** 暴露稳定 `id`、本地化 `title`、用户可见 `group`、启用条件说明、可选快捷键文本，以及 unavailable reason（若不可用）。
    *   Command Palette 与 Unified Search 的 `>` 命令入口 **MUST** 使用同一 command registry metadata；不得让两个入口展示不同的可用性或 writer 边界。
    *   File tree context menu、Command Palette、shortcut 与 toolbar 共享 `ContextAction` metadata 与 resolver；surface 只能用当前 `surface + target + readonly + repo scope + write readiness` 投影 `ProjectedContextAction` 并触发 `ContextActionIntent`，不得各自发明执行语义或提交裸 `action_id`。
    *   Context action intent **MUST** 携带 projection 时的 repo scope；handler / control bridge **MUST** 在分发前用当前 readiness / readonly / repo scope 调用 resolver；scope mismatch、write gate blocked 或 resolver miss **MUST** fail-closed 且无副作用。
    *   External action 只能作为 unavailable 或经 server/native adapter 明确启用的 action 出现；Web MUST NOT 直接执行 exe、script 或 shell command。
    *   `Export PDF` 若作为外部工具或脚本能力出现，默认只能注册为 dormant `ContextAction`；启用前 MUST 由后端/native adapter 提供 capability、绝对路径、timeout 与 output limit。

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

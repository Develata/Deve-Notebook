# 13_settings.md - 设置篇 (Settings)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Planned / Optional`
- `Counterpart Feature`: `docs/features/13_settings.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/11_commands_settings.md`
- `Primary Code Areas`: `crates/core/src/config.rs`, `apps/cli/src/commands/config.rs`, `apps/web/src/components/settings.rs`, `apps/web/src/hooks/use_layout.rs`

本章汇总系统所有配置项，包括环境变量、运行时配置文件 (`config.toml`) 以及快捷键映射。

当前权威状态以 `docs/plan/deve-note plan.md` 为准：本章是规划/扩展契约。当前已实现边界是
CLI/runtime 读取与写入 `config.toml` 的受支持键；server-backed Settings API 与统一 GUI
持久化状态仍是 future work，不得在验收中伪装成已完成能力。

## 1. Environment Variables (环境变量)

系统启动时支持的的环境变量列表：

| 变量名 (Key)                     | 默认值 (Default) | 说明 (Description)                                                  |
| :------------------------------- | :--------------- | :------------------------------------------------------------------ |
| **System Core**                  |                  |                                                                     |
| `DEVE_PROFILE`                   | `standard`       | 运行模式预设: `standard` (默认), `low-spec` (低配). |
| `DEVE_LEDGER_DIR`                | `ledger`         | 账本存储目录；Docker/runtime 推荐设为 `/data/ledger`。              |
| `DEVE_VAULT_PATH`                | `vault`          | 投影库根目录；Docker/runtime 推荐设为 `/data/vault`。               |
| `DEVE_SYNC_MODE`                 | `auto`           | 同步模式: `auto` 或 `manual`。                                      |
| `LOG_LEVEL`                      | `info`           | 日志级别: `trace`, `debug`, `info`, `warn`, `error`.                |
| `MEM_CACHE_MB`                   | `128`            | 内存缓存上限 (MB).                                                  |
| **Network & Security**           |                  |                                                                     |
| `AUTH_SECRET`                    | *(Random)*       | Session/JWT 签名密钥. **生产环境 MUST 设置**.                       |
| `AUTH_USER`                      | `admin`          | 默认超管用户名.                                                     |
| `AUTH_PASS`                      | *(none)*         | Argon2 哈希后的管理员密码（PHC 格式）。生产环境 MUST 设置。          |
| `AUTH_ALLOW_ANONYMOUS_LOCALHOST` | `false`          | 是否允许 Localhost / LAN 免密访问 (`true` / `false`).               |
| `ALLOWED_ORIGINS`                | *(none)*         | 允许的 CORS Origin 列表 (逗号分隔). 生产环境 MUST 显式设置。        |
| **AI**                           |                  |                                                                     |
| `AI_API_KEY`                     | *(none)*         | Native AI Chat 的服务密钥。                                         |
| `AI_BASE_URL`                    | `https://api.openai.com/v1` | Native AI Chat API 端点。                               |
| `AI_MODEL`                       | `gpt-4o-mini`    | Native AI Chat 默认模型。                                           |
| `AI_MAX_TOKENS`                  | `4096`           | Native AI Chat 输出上限。                                           |
| `AGENT_CLI_PATH`                 | *(none)*         | Trusted External Agent 可执行路径。未显式启用时不得读取。           |
| **TLS (可选)**                   |                  |                                                                     |
| `TLS_CERT_PATH`                  | *(none)*         | PEM 证书路径. 设置后启用 HTTPS.                                     |
| `TLS_KEY_PATH`                   | *(none)*         | PEM 私钥路径.                                                       |
| **Paths**                        |                  |                                                                     |
| `DEVE_DATA_DIR`                  | `~/.deve-note`   | 数据存储根目录.                                                     |

`DEVE_*` 扁平字段保留下划线命名；嵌套配置如后续需要通过环境变量覆盖，使用双下划线分隔层级。

## 2. Configuration Settings (config.toml) {#configuration-settings}

用户可配置的运行时选项存储在 `config.toml`，并可通过 `deve config print/set` 查看或更新。
浏览器本地 UI 偏好当前仍由前端本地状态/`localStorage` 管理；后续如引入独立设置文件
或 server-backed Settings API，必须先更新本章和验收用例。

### 2.1 UI Appearance (界面)
| Key                        | Type   | Default | Description                                         |
| :------------------------- | :----- | :------ | :-------------------------------------------------- |
| `ui.locale`                | String | `auto`  | 界面语言. 支持 `en-US`, `zh-CN`. `auto` 跟随浏览器. |
| `ui.theme`                 | String | `auto`  | 主题模式. `light`, `dark`, `auto`.                  |
| `ui.sidebar_visible`       | Bool   | `true`  | 是否显示 Primary Sidebar (左侧栏).                  |
| `ui.statusbar_visible`     | Bool   | `true`  | 是否显示 Status Bar (底部状态栏).                   |
| `ui.outline_visible`       | Bool   | `true`  | 是否显示 Outline Panel (右侧大纲).                  |
| `ui.outline_width`         | Number | `260`   | Outline 面板宽度 (Fixed, px).                       |
| `ui.sidebar_width`         | Number | `250`   | Sidebar 默认宽度 (Resizable, px).                   |
| `ui.right_panel_width`     | Number | `350`   | 右侧面板默认宽度 (Resizable, px).                   |
| `ui.outer_gutter`          | Number | `16`    | 主区域左右边距 (Resizable, px).                     |
| `ui.recent_commands_count` | Number | `3`     | Command Palette 顶部显示的历史命令数.               |
| `ui.recent_docs_count`     | Number | `10`    | Quick Open 顶部显示的历史文件数.                    |

### 2.2 Core Logic (核心)
| Key (config.toml / Env) | Type   | Default    | Description                                            |
| :---------------------- | :----- | :--------- | :----------------------------------------------------- |
| `profile`               | String | `standard` | 运行模式: `standard` (全功能), `low-spec` (低配).      |
| `ledger_dir`            | String | `ledger`   | 账本存储目录 (Relative or Absolute).                   |
| `vault_path`            | String | `vault`    | 投影库根目录 (Relative or Absolute).                   |
| `sync_mode`             | String | `auto`     | 同步模式: `auto` (自动合并), `manual` (接收后暂存，按单一 peer/repo 目标确认后原子合并). |
| `snapshot_depth`        | Number | `100`      | 快照保留深度 (Versions per Repo).                      |
| `concurrency`           | Number | `4`        | 后台任务并发数 (Compression/GC).                       |
| `merge_strategy`        | String | `manual`   | 冲突合并策略: `manual` (用户选择) \| `auto` (自动合并)。权威语义见 `07_diff_logic.md §Conflict Resolution`。 |

### 2.3 AI (人工智能)
| Key                        | Type   | Default      | Description |
| :------------------------- | :----- | :----------- | :---------- |
| `ai.mode`                  | String | `native`     | `native` = Native AI Chat；`trusted-cli` = 受信任外部 CLI（仅在显式启用时可选）。 |
| `ai.native_enabled`        | Bool   | `true`       | 是否启用 Native AI Chat。 |
| `ai.agent_bridge.enabled`  | Bool   | `false`      | 是否启用 Trusted External Agent。默认关闭。 |
| `ai.agent_bridge.trusted`  | Bool   | `false`      | 是否确认当前部署为受信任本地环境。未确认时 MUST NOT 起 CLI。 |
| `ai.agent_bridge.timeout_ms` | Number | `30000`    | 外部 CLI 单次请求超时。 |

补充约束：

*   `ai.mode = trusted-cli` 仅在以下条件全部满足时才有效：
    - `ai.agent_bridge.enabled = true`
    - `ai.agent_bridge.trusted = true`
    - `AGENT_CLI_PATH` 已设置为绝对路径，且目标存在并可执行
*   任一条件不满足时，系统 **MUST** 自动退回 `ai.mode = native`，并向用户显示明确原因。
*   `PLAN / BUILD` 是 Native AI Chat 的会话模式，不是单独的配置后端键。
*   Settings 中的后端切换与 Chat 内的 `/plan /build /agents` 必须明确分离，避免混淆“后端”与“模式”。

## 3. Keyboard Shortcuts (快捷键) {#keyboard-shortcuts}

| 场景 (Scope)          | 快捷键 (Mac / Win)             | 命令 (Command)                          |
| :-------------------- | :----------------------------- | :-------------------------------------- |
| **Global Navigation** | `Cmd+Shift+P` / `Ctrl+Shift+P` | **Command Palette**: 呼出全局命令面板   |
|                       | `Cmd+P` / `Ctrl+P`             | **Quick Open**: 快速跳转文件            |
|                       | `Cmd+Shift+K` / `Ctrl+Shift+K` | **Switch Branch**: 切换分支             |
|                       | `Cmd+Shift+O` / `Ctrl+Shift+O` | **Toggle Outline**: 开关右侧大纲栏      |
|                       | `Cmd+B` / `Ctrl+B`             | **Toggle Sidebar**: 开关左侧侧边栏      |
|                       | `Cmd+L` / `Ctrl+L`             | **Toggle Language**: 循环切换界面语言   |
| **Version Control**   | `Cmd+S` / `Ctrl+S`             | **Save**: 保存当前文件 (触发 Diff 计算) |
|                       | `Cmd+Enter` / `Ctrl+Enter`     | **Commit**: 提交暂存区的更改            |
|                       | `Cmd+A` / `Ctrl+A`             | **Select All**: 全选当前文件            |

## 4. Browser UI Preferences {#browser-ui-prefs}

浏览器本地 UI 偏好仅保存主题、布局、语言、最近命令等无害状态。`localStorage` 不可用时可以退回内存态，
但不得把 repo authority、session secret、peer private key 或业务事实写入该层。

当前实现要求所有前端 UI 偏好通过 `apps/web/src/storage/prefs.rs` 进入浏览器存储 fallback 层。
除 `storage/prefs.rs` 本身与 `storage/js_bridge.rs` 能力探测外，不得在功能模块中直接调用
`window.localStorage` / `sessionStorage`。布局宽度、Outline 可见性、语言偏好、快捷键覆盖等均属于
无害 UI prefs；repo identity、sync vector、writer readiness、scope nonce、auth secret 仍不得写入该层。
`deve.ui.last_scope` 只允许保存最后打开的 `repo_name` 显示别名，用于请求 server 重新解析；不得保存
`repo_id`、remote branch / peer id、`scope_nonce` 或任何可被当作 repo authority 的身份字段。

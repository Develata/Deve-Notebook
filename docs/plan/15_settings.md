# 15_settings.md - 设置篇 (Settings)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Planned / Optional`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-20`
- `Counterpart Feature`: `docs/features/13_settings.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/11_commands_settings.md`
- `Primary Code Areas`: `crates/core/src/config.rs`, `apps/cli/src/commands/config.rs`, `apps/web/src/components/settings.rs`, `apps/web/src/hooks/use_layout.rs`

本章汇总系统所有配置项，包括环境变量、运行时配置文件 (`config.toml`) 以及快捷键映射。

权威状态以 `docs/plan/deve-note plan.md` 为准：本章是规划/扩展契约。

配置面分为三类：

*   **Runtime Config Contract**：`config.toml` 与环境变量共同决定服务端运行时配置；受支持键 **MUST** 可由 CLI/runtime 读取，写入入口 **MUST** 校验 key、type 与敏感字段边界。
*   **Browser Preference Contract**：主题、布局、语言、最近命令等 UI 偏好 **MAY** 存入浏览器本地存储，但 **MUST NOT** 承载 repo authority、session secret、peer private key 或业务事实。
*   **Native Host-local Backend Preference Contract**：Desktop/Mobile native shell 的 backend 选择只允许存入 app-private data root 下的 host-local JSON 文件。它不是 `config.toml`、server-backed Settings API、Projection Locator、ledger fact 或 browser storage。
*   **Future Settings Surface**：server-backed Settings API、独立设置文件或统一 GUI 持久化 **MAY** 另行设计；启用前 **MUST** 更新本章、feature spec 与 acceptance case。

## 1. Environment Variables (环境变量)

系统启动时支持的的环境变量列表：

| 变量名 (Key)                     | 默认值 (Default) | 说明 (Description)                                                  |
| :------------------------------- | :--------------- | :------------------------------------------------------------------ |
| **System Core**                  |                  |                                                                     |
| `DEVE_PROFILE`                   | `standard`       | 运行模式预设: `standard` (默认), `low-spec` (低配). |
| `DEVE_LEDGER_DIR`                | `ledger`         | 账本存储目录；Docker/runtime 推荐设为 `/data/ledger`。              |
| `DEVE_SYNC_MODE`                 | `auto`           | 同步模式: `auto` 或 `manual`。                                      |
| `LOG_LEVEL`                      | `info`           | 日志级别: `trace`, `debug`, `info`, `warn`, `error`.                |
| `MEM_CACHE_MB`                   | `128`            | 内存缓存上限 (MB).                                                  |
| **Network & Security**           |                  |                                                                     |
| `AUTH_SECRET`                    | *(Random)*       | Session/JWT 签名密钥。生产环境 **MUST** 设置。                       |
| `AUTH_USER`                      | `admin`          | 默认超管用户名.                                                     |
| `AUTH_PASS`                      | *(none)*         | Argon2 哈希后的管理员密码（PHC 格式）。生产环境 **MUST** 设置。          |
| `AUTH_ALLOW_ANONYMOUS_LOCALHOST` | `false`          | development-only loopback 免密访问 (`true` / `false`)；仅在有效 development mode（`deve serve --dev` 或 `DEVE_ENV=development`）下可开启。 |
| `ALLOWED_ORIGINS`                | *(none)*         | 允许的 CORS Origin 列表 (逗号分隔)。生产环境 **MUST** 显式设置。        |
| **AI**                           |                  |                                                                     |
| `AI_API_KEY`                     | *(none)*         | Native AI Chat 的服务密钥。                                         |
| `AI_BASE_URL`                    | `https://api.openai.com/v1` | Native AI Chat API 端点。                               |
| `AI_MODEL`                       | `gpt-4o-mini`    | Native AI Chat 默认模型。                                           |
| `AI_MAX_TOKENS`                  | `4096`           | Native AI Chat 输出上限。                                           |
| `AGENT_CLI_PATH`                 | *(none)*         | Trusted External Agent 可执行路径。未显式启用时不得读取。           |
| `DEVE_AI_AGENT_BRIDGE_ENABLED`   | `false`          | `ai.agent_bridge.enabled` 的兼容环境变量别名。                      |
| `DEVE_AI_AGENT_BRIDGE_TRUSTED`   | `false`          | `ai.agent_bridge.trusted` 的兼容环境变量别名。                      |
| **TLS (可选)**                   |                  |                                                                     |
| `TLS_CERT_PATH`                  | *(none)*         | PEM 证书路径. 设置后启用 HTTPS.                                     |
| `TLS_KEY_PATH`                   | *(none)*         | PEM 私钥路径.                                                       |
| **Paths**                        |                  |                                                                     |
| `DEVE_DATA_DIR`                  | `~/.deve-note`   | 数据存储根目录.                                                     |
| `DEVE_GIT_EXECUTABLE`            | *(none)*         | 内部 native host → sidecar Git 绑定；必须为 absolute、存在的 ordinary file。显式无效时 Git 操作 fail-closed，不回退 PATH；它不是 `config.toml` 或 Source Control policy。 |
| `DEVE_GIT_EXECUTABLE_UNAVAILABLE` | *(none)*        | 内部 native host → sidecar unavailable marker；只接受 `1`，与 executable path 互斥，并禁止 sidecar 退回普通 Git 搜索。 |

`DEVE_*` 扁平字段保留下划线命名；嵌套配置如后续需要通过环境变量覆盖，使用双下划线分隔层级。

`DEVE_GIT_EXECUTABLE` 及其 `DEVE_GIT_EXECUTABLE_UNAVAILABLE=1` companion 是进程边界变量而非用户持久配置。普通 CLI 两者均未设置时可以按自身
`PATH` 解析 `git`；Desktop sidecar 因清空继承环境，只能使用宿主验证后传入的该绝对路径，
或接收互斥的 unavailable marker；不得继承完整 `PATH` / `PATHEXT`。变量缺失或无效只影响 Git bridge 可用性，不影响
LocalBackend 或 NoteGit authority。

## 2. Configuration Settings (config.toml) {#configuration-settings}

用户可配置的运行时选项存储在 `config.toml`，并可通过 `deve config print/set` 查看或更新。
浏览器本地 UI 偏好属于前端本地状态/`localStorage` 管理边界；若引入独立设置文件
或 server-backed Settings API，**MUST** 先更新本章和验收用例。
`deve config set` v1 只支持下表中的标量键写入；`p2p.peers[]` 这类数组配置可由
`config.toml`、init template 或环境变量声明，但不得被标量写入口伪装成数组 writer。

### 2.1 UI Appearance (界面)
| Key                        | Type   | Default | Description                                         |
| :------------------------- | :----- | :------ | :-------------------------------------------------- |
| `ui.locale`                | String | `auto`  | 界面语言. 支持 `en-US`, `zh-CN`. `auto` 跟随浏览器. |
| `ui.theme`                 | String | `warm`  | 视觉风格. `warm` (暖中性, 默认), `cold` (冷蓝白), `night` (夜间深色). 旧值迁移: dark→night, light/auto→warm. |
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
| `repo_creation_projection_base` | Optional absolute path | *(none)* | zero-repo host 创建首个 repo 时使用的默认 Projection Base；不是 workspace authority。 |
| `sync_mode`             | String | `auto`     | 同步模式: `auto` (自动合并), `manual` (接收后暂存，按单一 peer/repo 目标确认后原子合并). |
| `snapshot_depth`        | Number | `100`      | 快照保留深度 (Versions per Repo).                      |
| `mem_cache_mb`          | Number | `128`      | 内存缓存上限 (MB).                                      |
| `concurrency`           | Number | `4`        | 后台任务并发数 (Compression/GC).                       |
| `merge_strategy`        | String | `manual`   | 冲突合并策略: `manual` (用户选择) \| `auto` (自动合并)。权威语义见 `05_diff_logic.md §Conflict Resolution`。 |
| `p2p.enabled`           | Bool   | `false`    | 静态 FullPeer mesh 开关；默认关闭，启用边界见 `07_network.md#static-peer-config`。 |
| `p2p.inbound_token_env` | String | `DEVE_P2P_INBOUND_TOKEN` | 入站 FullPeer bearer token 的环境变量名；配置只保存非空 env 名称，**MUST NOT** 保存 token material。 |
| `p2p.connect_interval_ms` | Number | `5000`   | 静态 peer connector 重连间隔；配置值 **MUST** 大于 0，否则运行配置加载与 `deve config set` **MUST** fail-closed；实现 **MUST** 避免 busy loop。 |
| `p2p.peers[].label`     | String | *(none)*   | 运维可读显示名，只用于日志与 `/api/node/role` 诊断。不得作为身份校验输入。 |
| `p2p.peers[].peer_id`   | String | *(none)*   | expected authenticated peer identity；必须来自对端 PeerID 诊断/启动日志，不得填显示 label。 |
| `p2p.peers[].repo_id`   | UUID String | *(none)* | 静态 peer 共享的逻辑 `RepoId`；不同 repo 必须 fail-closed。 |
| `p2p.peers[].ws_url`    | String | *(none)*   | 对端 FullPeer `/ws` endpoint；scheme 必须为 `ws://` 或 `wss://`。 |
| `p2p.peers[].auth_token_env` | String | *(none)* | 出站 bearer token 的环境变量名；配置只保存 env 名称，**MUST NOT** 保存 token material。 |
| `p2p.peers[].enabled`   | Bool   | `true`     | 单个静态 peer connector 开关。 |

Source Control 不再暴露 `source_control.git_bridge` 或等价环境变量。NoteGit/ngit 是唯一
Source Control authority，Git main mirror 是固定外部生态投影；不得通过 Settings 把系统切换为
Git authority 或 NoteGit-only/off mode。

`p2p.peers[]` 中 `peer_id + repo_id + ws_url` 组成静态 peer identity tuple；重复 tuple 必须在
runtime config 加载时 fail-closed，避免多个 peer entry 共享同一 connector 诊断状态。
通过环境变量声明 `p2p.peers[]` 时，peer index 必须从 `0` 连续递增；出现稀疏 index 或只有部分字段的
peer entry 必须在配置加载时 fail-closed，不能静默忽略后续 peer。

`profile` 只提供默认预设。显式 `config.toml` 或环境变量 **MUST** 覆盖 profile preset；未显式设置时，`low-spec` **MUST** 使用 `snapshot_depth = 10`、`MEM_CACHE_MB = 32`，`standard` **MUST** 使用 `snapshot_depth = 100`、`MEM_CACHE_MB = 128`。

### 2.2.1 Projection Locator Settings

已存在 repo 的 projection base / workspace root 不属于 `config.toml` 的全局键。

规则：

*   系统 **MUST NOT** 支持 `vault_path` / `DEVE_VAULT_PATH` 作为全局投影根。
*   `repo_creation_projection_base` 只在没有 current local locator 可复用时，为 Create 提供一个显式 absolute parent base。它不得覆盖既有 locator、推导 workspace root 或参与 repo identity；zero-repo host 缺失该键时仍可启动，但 Create 返回 `REPO_CREATION_PROJECTION_BASE_REQUIRED`。
*   每个本地可写 repo 的 projection base 与 immutable `workspace_segment` 必须通过 host-local Projection Locator 绑定；最终 workspace root 为 `<projection_base>/<workspace_segment>/`。当前 alias 不参与路径计算。locator 存储边界见 `03_storage/projection.md#projection-locator-contract`。
*   `config.toml` 可以决定 `ledger_dir`，但不得通过 `ledger_dir` 推导 projection base 或 workspace root。
*   Settings UI 或 CLI 可以展示、创建、替换 locator；写入前 **MUST** 校验 path 类型、canonical path、冲突与保留目录边界。
*   locator 变更属于 repo runtime 操作，不是用户 UI 偏好，也不是 ledger authority。

### 2.2.2 Native Host-local Backend Preference {#native-host-local-backend-preference}

Desktop/Mobile native shell 可以在 Settings 暴露 Backend section，但其持久化边界独立于服务端 Settings、浏览器 UI 偏好与 repo authority。

规则：

*   持久化模型为 `NativeBackendPreference { mode: local | remote, remote_url }`，默认 `local`。
*   持久化位置必须是 app-private data root 下的 host-local JSON 文件，例如 `native-backend.json`；不得写入 `config.toml`、ledger、Projection Locator、projection workspace、browser `localStorage` 或 `sessionStorage`。
*   `local` 模式不得要求用户手动启动后端：Desktop 启动受控 `deve_cli serve --native-loopback` 子进程，Android/Mobile 启动 embedded loopback service。
*   `remote` 模式只保存已校验 HTTPS origin，并且不得启动本机后端或注入本地 endpoint/session bootstrap。
*   保存 `remote` 前必须由 native 侧短超时请求 `<origin>/api/node/role`，确认响应是结构化 Deve node role；校验失败不得保存。远端认证、cookie 与登录态由远端 Web 自行处理。
*   该 preference 不得保存远端凭证、session/token、native session material、P2P token、repo id、branch、`scope_nonce`、writer readiness 或任何 authority fact。
*   普通浏览器与 native `RemoteBrowser` 中的 Settings 必须把 Backend section 标为 native-only unavailable，不提供伪保存、伪校验、Web “Use local backend”或写浏览器存储的替代路径。只有 typed bundled-local bootstrap 明确授予 `backend_preference_control` 时才可注册 Web facade。
*   native `RemoteBrowser` 失联时仍按浏览器断连锁屏/只读语义处理；切回 `local` preference 属于 native-owned lifecycle 操作。Desktop 由原生菜单/托盘 coordinator 承载；Mobile 由 Android/iOS 平台原生控件与 native coordinator 承载。Mobile 控件只在 preference-driven RemoteBrowser 中可用，显式 CLI/env override 下必须隐藏；两者都不得向远端页面开放 IPC，也不得复用远端 session/scope。

### 2.3 AI (人工智能)
| Key                        | Type   | Default      | Description |
| :------------------------- | :----- | :----------- | :---------- |
| `ai.mode`                  | String | `native`     | `native` = Native AI Chat；`trusted-cli` = 受信任外部 CLI（仅在显式启用时可选）。 |
| `ai.native_enabled`        | Bool   | `true`       | 是否启用 Native AI Chat。 |
| `ai.agent_bridge.enabled`  | Bool   | `false`      | 是否启用 Trusted External Agent。默认关闭。 |
| `ai.agent_bridge.trusted`  | Bool   | `false`      | 是否确认当前部署为受信任本地环境。未确认时 **MUST NOT** 起 CLI。 |
| `ai.agent_bridge.timeout_ms` | Number | `30000`    | 外部 CLI 单次请求超时。 |

补充约束：

*   `ai.mode = trusted-cli` 仅在以下条件全部满足时才有效：
    - `ai.agent_bridge.enabled = true`
    - `ai.agent_bridge.trusted = true`
    - `AGENT_CLI_PATH` 已设置为绝对路径，且目标存在并可执行
*   任一条件不满足时，系统 **MUST** 将 effective backend 自动退回 `native`，并向用户显示明确原因；`config.toml` 中用户请求的 `ai.mode` 不应被静默改写。
*   `ai.native_enabled = false` 时，Native AI Chat **MUST NOT** 注册 provider 或接受 `ai-chat` RPC；前端能力探测必须把 Native backend 标为不可用。
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
| **Editor**            | `Cmd+Z` / `Ctrl+Z`             | **Undo**: 撤销当前编辑器会话内一步编辑  |
|                       | `Cmd+Shift+Z` / `Ctrl+Y`       | **Redo**: 重做当前编辑器会话内一步编辑  |
| **Version Control**   | `Cmd+S` / `Ctrl+S`             | **Save**: 保存当前文件 (触发 Diff 计算) |
|                       | `Cmd+Enter` / `Ctrl+Enter`     | **Commit**: 提交暂存区的更改            |
|                       | `Cmd+A` / `Ctrl+A`             | **Select All**: 全选当前文件            |

## 4. Browser UI Preferences {#browser-ui-prefs}

浏览器本地 UI 偏好仅保存主题、布局、语言、最近命令等无害状态。`localStorage` 不可用时可以退回内存态，
但不得把 repo authority、session secret、peer private key 或业务事实写入该层。

Settings v1 最小 UI surface：

*   外观：视觉风格 `warm` / `cold` / `night`（默认 `warm`），只写浏览器本地 `deve.ui.theme`，并通过根节点主题标记 `data-deve-theme-pref` 提供即时反馈。三种风格是平级命名风格而非 light/dark 二元开关：`warm` 暖中性纸面、`cold` 冷蓝白、`night` 夜间深色；`night` 同时设置 `color-scheme: dark`。启动期 MUST 在 CSS 绘制前由 pre-paint bootstrap resource 设置该标记以避免主题闪烁；HTML shell 只负责按顺序加载资源，不直接承载 browser storage 逻辑。WASM 启动后仍以浏览器本地 `deve.ui.theme` 为准重放。旧值按 `dark→night`、`light`/`auto→warm` 迁移；未知值回退默认 `warm`。
*   编辑器：自动换行、编辑器密度与最大文档标签页数只作为本地 UI 标记，不写 server-backed settings，不改变 repo 文档事实。
    *   最大文档标签页数 key 为 `deve.ui.max_document_tabs`，默认值 `8`，有效范围 `1..=20`。
    *   非法值必须回退默认值；合法但越界的用户输入必须 clamp 到有效范围。
    *   数字输入编辑过程使用 draft value；只有 blur / Enter / change 这类显式提交点才能更新
        `MaxDocumentTabs` runtime state，避免两位数输入过程短暂触发过小上限并自动淘汰标签页。
    *   该值只控制 UI shell tab registry 的 `DocumentTab` 自动淘汰，不持久化打开文档列表、visible tab order 或 document access order。
*   运行诊断：可展示 embedded browser 与 Trunk fallback smoke 入口；这些入口只指向本地 runbook/script，不启动后台 writer。

所有前端 UI 偏好必须通过 browser storage prefs facade 进入浏览器存储 fallback 层。
除该 facade 本身与底层能力探测外，不得在功能模块中直接调用
`window.localStorage` / `sessionStorage`。布局宽度、Outline 可见性、语言偏好、快捷键覆盖等均属于
无害 UI prefs；sync vector、writer readiness、scope nonce、auth secret 仍不得写入该层。最近 scope 可以保存 RepoId 作为恢复提示，但它不是 authority，server 必须重新 admission。
快捷键覆盖这类结构化 UI prefs 的新写入必须使用 JSON 序列化/反序列化 API；不得用手写字符串拼接或分隔符协议保存结构化偏好。旧版分隔符格式只能作为读取迁移兼容。
`deve.ui.last_scope` 只允许保存最后打开的 canonical `repo_id` 与 local branch kind，用于请求 server 重新解析；不得保存 alias、remote peer id、`scope_nonce` 或任何 writer grant。alias 由 server 的 host-local runtime 每次投影，浏览器不得把旧 alias 回写为选择 authority。
AI Chat 面板可见性属于 browser-local UI pref（当前 key 为 `deve.ui.ai_chat_visible`）；Settings 隐藏 AI Chat 时只影响 Chat surface 与分界线是否渲染，
不得关闭 Native AI provider、修改 `ai.mode`、清空聊天历史或改变 repo/document/source-control authority。

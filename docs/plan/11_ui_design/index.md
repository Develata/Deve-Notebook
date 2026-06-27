# 11_ui_design/index.md - UI Shell 与 Application Control 工程蓝图

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Version`: `0.0.1`
- `Last Review`: `2026-06-26`
- `Counterpart Feature`: `docs/features/08_ui_design.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/05_ui.md`, `docs/acceptance-cases/13_ui_mobile_chat_regression.md`
- `Primary Code Areas`: `apps/web/src/context_action/`, `apps/web/src/components/`, `apps/web/src/hooks/use_core/callbacks*.rs`, `apps/web/src/hooks/use_core/navigation.rs`, `apps/web/src/components/mobile_layout/`

> **Modules**: [Web](./01_web.md) | [Desktop](./02_desktop.md) | [Mobile](./03_mobile.md)

## 1. Scope

本章不是视觉说明书，而是 UI shell 的工程蓝图。

本章定义：

- 显示层、application control 层、runtime 层之间的依赖方向
- 多端共享控制接口
- shell / panel / drawer / overlay / command 的状态机
- 显示层的禁区

用户点击后看到什么、按钮文案与 Chrome MCP 手工实例属于 `docs/features/08_ui_design.md`。

## 2. Design Invariants

### 2.1 Thin View Rule

- 显示层 MUST 只负责展示与收集输入。
- 显示层 MUST NOT 直接改写 repo scope、document authority、pending write、source control side table。
- 所有业务操控必须经由 application control、runtime command 或 CLI / server command。

### 2.2 Command First

- 所有核心用户能力 MUST 可映射到稳定的 `CommandId`。
- 按钮、菜单、快捷键、移动端动作只是同一 command 的不同触发器。

### 2.3 Multi-Surface Consistency

- Web / Desktop / Mobile 共享同一 feature runtime 与 command 语义。
- 各端只允许在 shell、viewport、gesture、platform adapter 层不同。

### 2.4 Runtime Ownership

- shell state 归 UI shell runtime 所有。
- document state 归 document runtime 所有。
- source control state 归 source control runtime 所有。
- repo/branch state 归 scope runtime 所有。

禁止一个 view 组件同时持有多个 runtime 的私有真相副本。

### 2.5 Responsive and Multi-Surface Mapping

- Web 端在移动端视口 MUST 匹配 Mobile shell contract。
- Web 端在大屏视口 MUST 匹配 Desktop shell contract。
- Desktop / Mobile 外壳可以不同，但 command/control/runtime 语义必须一致。

## 3. Authoritative Entities

### 3.1 View Entities

- `ViewId`: Explorer / Search / SourceControl / Extensions / Chat / Settings / Diff / Dashboard
- `PanelId`
- `DrawerId`
- `OverlayId`
- `CommandId`

### 3.2 Layout Entities

- `SidebarWidth`
- `RightPanelWidth`
- `DisplayEditorWidth`
- `AiChatVisibility`
- `MaxDocumentTabs`
- `OuterGutterWidth`
- `ActivityBarPinState`
- `FocusedSurface`

这些都是 UI prefs，不是业务 authority。

### 3.3 Application Control

application control 是连接 view 与 runtime 的稳定接口层，至少包含：

- `OpenDocument`
- `SwitchRepo`
- `SwitchBranch`
- `RequestChanges`
- `StageTarget`
- `CommitStaged`
- `OpenDiff`
- `OpenCommandPalette`
- `ToggleSidebar`
- `ToggleDrawer`

控件只能发出这些控制意图，不能绕过它们直接修改 runtime 内部信号。

### 3.3.1 Context Action Surface {#context-action-surface}

Context action 是 command/control 体系在具体对象上的投影，不是某个菜单自己的注册表。

四层调用链：

- `User Operation`：用户在 file tree、command palette、toolbar 或 shortcut 上选择动作。
- `Instruction Interface`：surface 只展示 action metadata，收集 target 与用户输入。
- `Flow Coordination`：通过 Context Action resolver 统一解析 `action_id`、surface、target、repo scope、readiness、request id、幂等与失败状态。
- `Execution Domain`：后端、server runtime 或 native adapter 执行 authority write、shell-local action 或受控 external action。

约束：

- file tree context menu MUST 只消费 `ContextActionDescriptor` 的 projection，不得把业务执行逻辑写进菜单渲染层。
- Web 端 `ContextAction` registry MUST 归属 application/control 层（当前为 `apps/web/src/context_action/`），不得归属 `sidebar_menu` 等单一 view 组件。
- projection MUST 以 `surface + target + readonly + repo scope + write readiness` 请求建模，并通过 resolver 生成 `ProjectedContextAction`；surface 只能消费 projection result，不能自行枚举完整能力表。
- UI surface MUST 只渲染 `ProjectedContextAction` 并在触发时提交 `ContextActionIntent`；不得向控制层提交裸 `action_id`。
- `ContextActionIntent` MUST 携带创建 projection 时的 repo scope；handler / control bridge MUST 用当前 readiness、readonly 与 repo scope 构造 resolve request 并调用 `resolve_context_action(...)` 二次裁决。
- resolver MUST 拒绝 stale scope、surface mismatch、target mismatch、untrusted external origin 与当前 write gate blocked 的写操作；resolve miss MUST fail-closed 且无副作用。
- Context Action resolver 属于 Flow Coordination，不属于 Execution Domain；它只裁决 intent 是否可分发，不执行 authority write、外部进程或脚本。
- 同一用户能力在 file tree、command palette、shortcut 中出现时，MUST 共享稳定 `action_id` 与可用性语义。
- Web surface MAY 进行纯展示过滤，例如 target kind、read-only display 与 external icon，但不得决定 external executable 是否可信。
- `ExternalProcess` action MUST 默认不可用；启用前必须由 server/native adapter 根据配置、capability、绝对路径、timeout 与 output limit fail-closed。
- `ExportPdf` 可作为 dormant `ExternalProcess` descriptor 注册在 Markdown target 上，但在 server/native adapter 明确启用前 MUST NOT 被 Web 投影为可执行 action。
- 外部动作图标只是用户可见 provenance 信号，不是安全边界。
- `ShellLocal` action 只能改变浏览器 shell 状态，例如打开新窗口；不得写 document、repo、source-control 或 projection authority。

### 3.4 Layout Tokens and Layer Registry

- design tokens MUST 通过 CSS variables 暴露，禁止 view 组件散布硬编码色值当作跨模块契约。
- shell 分层至少保留以下 z-index registry：
  - `Editor`
  - `Chrome`
  - `Panels`
  - `Floating`
  - `Overlay`
  - `Modal`
- `Toast`
- 不同 feature view 不得私自发明更高层级绕过统一层级表。

最小 canonical token 集：

- source control semantic colors
  - `--color-added`
  - `--color-modified`
  - `--color-deleted`
- shell layer tokens
  - `--z-editor`
  - `--z-chrome`
  - `--z-panels`
  - `--z-floating`
  - `--z-overlay`
  - `--z-modal`
  - `--z-toast`

### 3.5 Iconography Contract

- iconography 必须是稳定的 shared registry，不得让各 feature 随意混用不同图标语义。
- 同一动作在多端应共享同一 icon id / semantic name，例如：
  - `repo`
  - `branch`
  - `source_control`
  - `search`
  - `settings`
  - `diff`
- 图标选择属于 shell design token contract，而不是单个 view 自由发挥。

### 3.6 Editor Group Tabs {#editor-group-tabstrip}

主编辑区上方必须支持 desktop-style editor group tab strip，用于在多个已打开 surface
之间快速切换。该能力对齐 VS Code / Obsidian 的 workbench mental model，但只复用交互抽象，
不得复制实现、DOM、CSS 或产品资产。

Tab surface 类型：

- `DocumentTab`
  - 绑定稳定 `DocId`。
  - title 来自当前 repo 的 document display path。
  - 点击 tab 必须发出 `OpenDocument` / document navigation intent，并继续经过 pending
    navigation guard。
- `DiffTab`
  - 绑定 `DiffSession` 的稳定 identity（优先 `doc_id`，否则使用 canonical path +
    session timestamp 的 view-local key）。
  - title 使用 `display_path`，canonical path 只作为 selector / tooltip 辅助信息。
  - 点击 tab 只能恢复已有 view-local diff session；新的 diff 计算仍必须由
    source-control runtime 输出驱动。

状态边界：

- open tab list 与 active tab 归 UI shell runtime / view-local state 所有，不是
  document authority、source-control authority 或 repo scope authority。
- editor group 必须区分显式可见顺序与文档访问顺序：
  - visible tab order 只决定 desktop tab strip 的渲染和拖拽排序。
  - document access order 是隐藏 LRU 顺序，只用于文档 tab 上限淘汰。
  - 点击或打开 `DocumentTab` 必须 touch document access order；拖拽 tab 只改变 visible tab order。
  - visible tab order 的拖拽目标包括可见 tab 本身与 tab strip 末尾的可见空白区域；末尾空白 drop
    必须等价于移动到最后一个可见 tab 之后。
- repo / branch / scope 切换时，tab registry 必须清理或按 scope 隔离，禁止 stale tab
  在新 scope 中重新打开旧 `DocId` / diff。
- `DocumentTab` 数量超过 browser-local `MaxDocumentTabs` 时，必须自动关闭 LRU 最旧且非 active
  的 document tab；`DiffTab` 不参与该上限，也不得被 document tab 上限淘汰。
- 关闭 active document tab 必须等价于一次受 guard 保护的离开当前文档；若仍有相邻
  document tab，则切换到相邻 tab，否则回到 Dashboard。
- 关闭 active diff tab 只关闭 diff surface，不得修改 staged / pending / commit state；
  若仍有相邻 diff tab 可切换到相邻 diff，否则回到当前 document 或 Dashboard。
- Diff view header 可保留 hunk navigation、edit/preview、read-only 等 diff-local
  控件，但文件级切换必须归 editor group tab strip，不得在 diff header 中另起第二套
  文件切换状态。

最小工程蓝图：

- `desktop_layout/content` 只组合 `EditorTabStrip`、`Editor`、`DiffView` 与 `Dashboard`，
  不直接保存 tab registry 或实现关闭/切换状态机。
- `components/editor_tabs/runtime` 持有 view-local tab registry、visible tab order 与 document
  access order，订阅 `docs`、`current_doc`、`diff_content`、`current_repo_id`、
  `scope_nonce` 与 `MaxDocumentTabs`，并输出 typed callbacks。
- `components/editor_tabs/model` 只定义 tab identity、title / tooltip projection。
- `components/editor_tabs/ops` 只提供纯函数 upsert / remove / fallback neighbor 选择、
  visible order reorder 与 document LRU eviction。
- `desktop_layout/editor_tabs/strip` 只渲染 tablist 与 button semantics，不直接写 document、
  source-control 或 repo authority。

调用关系：

```text
DesktopLayoutContent
  -> tab_runtime callbacks
  -> navigation guard / existing core signals
  -> document runtime or diff session restore

DesktopLayoutContent
  -> EditorTabStrip
  -> typed select / close callbacks
```

输入输出与生命周期：

- 输入：当前 repo/scope、document list、active document、active diff session、pending local edit
  状态。
- 输出：`OpenDocument` 等价的 guarded document navigation、已有 diff session restore、view-local
  tab close。
- repo / branch / scope 变化时 runtime 必须清理 tab registry 与 active diff session。
- document 切换时 active diff session 必须退出，避免 diff surface 伪装成新 document authority。

失败、配置与性能：

- pending edit guard 阻止离开当前 active document 时，close / select 只能进入 pending navigation，
  不得提前删除 active document tab。
- 缺失 document projection 时不创建 document tab；diff session 只恢复已有 session，不触发新计算。
- 只持久化 `MaxDocumentTabs` browser-local UI pref；open tab list、visible order 与
  document access order 仍是 view-local state，reload 后可丢失。
- upsert / remove 是 `O(open_tabs)`；open tabs 是小规模 view-local shell state，不在关键 authority
  路径上。

## 4. Architecture Layers

### 4.1 Layer Order

```text
View / Widget / Shell
  -> Application Control
  -> Feature Runtime
  -> Authority / Projection / Transport
```

反向依赖是禁止的。

### 4.2 View Layer

包含：

- activity bar
- sidebar
- mobile layout
- overlays
- diff view
- chat panel

职责：

- 呈现 runtime 状态
- 转发 typed intents
- 管理纯 UI 本地态（开关、尺寸、hover、焦点）

### 4.3 Application Control Layer

包含：

- command routing
- layout command dispatch
- typed callback builders
- cross-surface intent normalization

职责：

- 把按钮/快捷键/手势统一成 feature runtime 可消费的命令
- 不直接保存业务 authority

### 4.4 Feature Runtime Layer

包含：

- `use_core` 中的 repo scope runtime
- document runtime
- source control runtime
- sync/session runtime

职责：

- 状态机
- ws/http/cli 请求
- ack/reject
- repo-scoped gating

### 4.5 Component Topology

建议目录职责：

```text
components/
  activity_bar/
  sidebar/
  mobile_layout/
  diff_view/
  chat/
  overlay/
  shared/
```

要求：

- 区域性 shell 组件按 layout 分区
- 原子组件不得偷偷携带 repo/document/source-control runtime 行为

## 5. Shell State Machines

### 5.1 Main Shell

```text
Booting
  -> Dashboard
  -> RepoScopedShell
  -> ModalOverlay
```

约束：

- `Dashboard` 与 `RepoScopedShell` 是两个不同的 shell state。
- overlay 打开时必须显式记录 `FocusedSurface` 与返回目标。

### 5.2 Focus State {#layout-navigation-and-focus}

```text
Editor
Sidebar
Panel
Modal
Drawer
```

规则：

- Modal MUST trap focus。
- 关闭 Modal 后 MUST restore previous focus target。
- 移动端 drawer 与 desktop sidebar 是不同 focus surface。
- 任一 surface 在被折叠到 `aria-hidden=true` 前，如果当前 active element 位于该 surface 内部，
  shell 必须先释放或迁移焦点，禁止隐藏区域保留焦点。
- 折叠 surface 的 DOM 隐藏必须使用两阶段状态：layout size 可先到 `0px`，但 `aria-hidden` /
  `visibility:hidden` 只能在焦点释放或确认焦点不在该 surface 内之后切换。

补充：

- `FocusedSurface` 是 shell runtime state，不是某个 view 自己猜测的 DOM 状态。
- modal / command palette / overlay 关闭后必须恢复到上一个稳定 surface。

### 5.3 Mobile Drawer State

```text
Closed
LeftDrawerOpen
RightDrawerOpen
ChatSheetOpen
```

规则：

- 边缘手势不得抢走交互控件点击。
- drawer 关闭必须清理 gesture capture。
- 移动端的更多菜单、repo switcher 与 source control menu 必须是 typed button semantics，而不是普通 div 点击。

### 5.4 Activity Bar / Menu State

```text
Collapsed
Expanded
PopupMenuOpen
PinnedSetChanged
```

规则：

- “切视图” 与 “Pin/Unpin” MUST 是两种独立控制语义。
- 菜单项整行点击只能触发 view switch，不能偷偷改 pin 状态。

### 5.5 Config Persistence Contract

- `deve_config` / layout prefs 只能存储纯 UI 偏好：
  - theme
  - sidebar visibility
  - widths
  - AI Chat panel visibility
  - maximum document tabs
  - language
  - recent active view
- `deve_config` MUST NOT 持有：
  - session token
  - peer identity
  - repo vector
  - pending writes
  - `scope_nonce`
  - `client_op_id`

## 6. Layout Persistence Contract

- 所有 layout 持久化都属于 `deve_config` / local UI prefs。
- localStorage 中只允许保存：
  - 侧栏宽度
  - 中间显示/编辑区宽度
  - 面板宽度
  - AI Chat 面板可见性
  - 最大文档标签页数
  - 主题
  - 最近活动 view
  - 语言
- 禁止存储：
  - repo vector
  - pending writes
  - peer identity
  - auth token
  - scope nonce

## 7. Interaction Contracts

### 7.1 Repo / Branch Switching

- view 层只能请求 `SwitchRepo` / `SwitchBranch`。
- 成功与失败反馈来自 scope runtime，不允许 view 层自行假设切换完成。

### 7.2 Document Navigation

- 导航前是否可离开当前文档，必须由 document runtime 根据 pending write state 决定。
- view 层只能展示 pending navigation modal，不得自行判断“是否有未确认写入”。

### 7.3 Source Control Actions

- stage / unstage / discard / commit 都必须经过 source control runtime gate。
- remote readonly branch MUST 在 control 层就进入只读，不得等 view 层“按钮按了没反应”。

### 7.4 Markdown Shell Actions

- outline、preview、open doc、diff navigation 都必须读取 runtime 投影，不得自己重建 authority state。

### 7.5 Command Routing Contract

- 所有核心能力 SHOULD 同时具备：
  - toolbar/button trigger
  - keyboard shortcut trigger（若适用）
  - command palette trigger
- 这三个入口最终必须映射到同一 `CommandId` / application control。
- file tree context menu trigger 属于同一 command/control 投影；新增 action 时必须先进入 `ContextActionDescriptor`，再由具体 surface 过滤展示。

## 8. Failure / Recovery

### 8.1 UI Recovery

- 页面 reload 后可恢复 layout prefs，但 MUST NOT 恢复过期的业务 authority。
- stale repo scope、stale diff session、stale pending navigation 必须通过 runtime 明确清理。

### 8.2 Runtime Failure Surfacing

- 未认证、断网、只读、repo scope mismatch、pending write reject 都必须来自结构化 runtime 状态。
- view 层禁止把任意 `String` 文本 error 当成稳定协议。

### 8.3 Multi-Surface Degrade

- Desktop / Mobile / Web 外壳差异只允许影响 shell 行为，不得改写 authority contract。
- Web 断连可降级只读；Desktop/Mobile 离线可继续使用本地 authority，但 UI 仍通过同一 command/control 体系驱动。

### 8.4 Offline / Thin-Client Split

- Web 是 thin client：
  - disconnect 后可降级只读
  - 不得继续伪装本地 authority 可写
- Desktop / Mobile native 可在本地 authority 上继续运行：
  - 但 view 层仍不能绕过 application control 直接写业务状态

### 8.5 Native Adapter Gate Registry {#native-adapter-gate-registry}

- `native_adapter/` 是 shell adapter 层，不是 authority core。
- Desktop / Mobile native shell **MUST** 支持两个互斥模式：
  - `LocalBackend`：native-packaging 运行时默认模式。壳层启动或持有本机受控后端，并在 app-private ledger/repo/projection 上完成初始化；所有读写仍经本机 server/core writer gate。
  - `RemoteBrowser`：显式远端模式。壳层只把 WebView 导航到远端 Docker/Web 的 HTTPS origin，行为等价于浏览器访问该站点；不得注入本地 endpoint/session bootstrap，不得启动本机后端。
- Desktop / Mobile native shell **MUST** 共用同一套 `NativeBackendPreference` 语义：默认 `local`；Settings 可切换 `local` / `remote`；`remote` 必须保存已校验 HTTPS origin；该偏好只允许持久化在 app-private data root 下的 host-local JSON 文件（例如 `native-backend.json`）。
- native backend preference 不是 repo authority、不是 `config.toml`、不是 Projection Locator，也不是 browser `localStorage` / `sessionStorage`；它不得保存 session、token、`scope_nonce`、peer secret 或 writer readiness。
- Desktop / Mobile 子章 **MAY** 定义平台 endpoint、session handoff、lifecycle、foreground reprobe 与 local backend 承载差异。
- Desktop / Mobile 子章 **MUST NOT** 引入新的 ledger、Projection Workspace、source-control、sync、search 或 settings authority。
- native adapter **MUST** 保持 no-packaging-runtime 默认构建；`native-packaging` 构建进入 `LocalBackend` 默认模式，而默认 workspace/no-Tauri 构建仍不含 packaging runtime。
- Desktop `LocalBackend` 使用受控 child-process local service；Mobile/Android `LocalBackend` 使用 embedded loopback service，Mobile v1 不使用子进程。
- native adapter 通过 gate 后 **MUST** 继续服从 writer gate、repo scope gate 与本章 control/runtime 分层；shell 不得直接写 ledger、projection、source-control、sync、search 或 settings authority。

### 8.6 Native Post-Gate Common Contract {#native-post-gate-common-contract}

- post-gate native shell **MUST** 先解析 `NativeShellMode`，再启动 UI。
- native shell **MUST** 先读取启动覆盖项与 host-local backend preference，再解析 `NativeShellMode`：CLI `--remote-url` / `DEVE_NATIVE_REMOTE_URL` 仅作为诊断或脚本覆盖；无覆盖时使用 host-local preference；无配置时默认 `LocalBackend`。
- `LocalBackend` **MUST** 先拉起受控本机 service 并完成 native session handoff；Desktop local service 与 Mobile embedded loopback service 都只是本机 full peer authority 的承载进程/循环，不改变业务写入入口。
- `RemoteBrowser` **MUST** 只接受 HTTPS origin URL；不接受 userinfo、query、fragment 或业务子路径。RemoteBrowser 壳层不拥有本机 repo、ledger 或 writer gate，只消费远端 Web 同源 `/api` 与 `/ws`。
- Settings 保存 `RemoteBrowser` 前 **MUST** 由 native 侧短超时探测 `<origin>/api/node/role` 并取得结构化 Deve node role；校验失败不得写入 host-local preference。远端登录态仍由远端 Web 自行管理，native bridge 不保存远端凭证。
- native Settings bridge **MUST** 只暴露窄接口：读取当前 backend preference、校验 remote origin、保存 remote preference、切回 local preference。普通浏览器必须显示 native-only unavailable，不提供伪操作。
- `RemoteBrowser` 失联时 UI 语义等价浏览器断连锁屏/只读；native shell 可以额外提供“Use local backend”入口，触发 host-local preference 切回 `LocalBackend`、启动本机 service，并重载 Web shell。切换过程不得复用旧 endpoint、旧 session、旧 repo scope 或旧 `scope_nonce`。
- service 端口 **MUST** 使用本机随机可用端口，并只保存在运行时内存中。
- 端口占用时，service boot **MUST** 自动回退到新的可用端口并重新绑定。
- 本机通信 **MUST** 使用 loopback HTTP/WS 或显式 IPC，并具备进程级鉴权与 session 绑定。
- Tauri custom-protocol CORS allowlist **MUST** 覆盖 Windows/Android 的 `http://tauri.localhost` 与 macOS/iOS/Linux 的 `tauri://localhost` 两种 origin 形式，不得只按单平台 origin 放行。
- Mobile embedded same-process `LocalBackend` **MUST** 通过 typed runtime launch options 传递 auth/session bootstrap material；**MUST NOT** 通过修改进程级环境变量把密钥交给同进程后端。
- 本机 service **MUST NOT** 监听非回环地址。
- 无公网时，`LocalBackend` 本地读写能力 **MUST** 仍由 core/server authority 与 writer gate 决定；`RemoteBrowser` 则等价于普通浏览器断连。
- native shell **MUST NOT** 直接写 ledger、projection、source-control、search、`.git` 或 `.notegit`；所有写入仍经本地 server/core writer gate。
- service port、session secret、P2P token material 与 bootstrap secret **MUST NOT** 写入 URL、日志、Web localStorage 或持久 config。
- 本地持久化、schema migration、repair、projection writeback、crash recovery 与本地内容落盘 **MUST** 服从 `03_storage/`。
- at-rest encryption、key rotation、key recovery 与进程级鉴权 **MUST** 服从 `08_auth.md`。
- 备份、导出、关键操作审计与恢复演练 **MUST** 服从 `18_release.md`。
- 本地存储安全、备份、导出、审计与恢复演练 **MUST NOT** 落入 UI view 层。
- native packaging 依赖 **MUST** 只落在对应 adapter feature scope。
- 启动速度、输入延迟与内存预算 **MUST** 优先于视觉特效。

## 9. Forbidden Patterns

- 组件直接写 `use_core` 内部信号以跳过 control 层。
- 菜单项点击既改 view 又改 pin。
- 为单个 file tree 菜单项绕过 context action/control 层直接绑定业务执行。
- view 组件直接调用底层 repo/source control/storage API。
- 把业务真相塞进 localStorage。
- 让 shell 组件自行猜测 repo writable / readonly 状态。
- 用普通 `div` 冒充需要 button semantics 的交互控件。

## 10. Runtime Boundary

### 10.1 View / Shell Layer

- 负责可见 shell、panel、drawer、overlay、diff/chat surface 与可访问性交互。
- 只能消费 runtime state 并发出 typed intent。

### 10.2 Application Control Layer

- 负责 command dispatch、navigation guard、write gate、repo/document/source-control intent 编排。
- 不得持有 authority state 的私有副本。

### 10.3 Feature Runtime Layer

- 负责 feature-specific state machine、server message dispatch、pending overlay、diff session lifecycle。
- runtime 间交互必须通过稳定 command/control surface，不得互相篡改内部状态。

### 10.4 Shared Layout Infrastructure

- 负责 layout prefs、panel dimensions、overlay host 与跨 surface focus/stacking 管理。
- 不得保存 repo authority、session secret、peer private key 或业务事实。

## 11. Refactor Target

长期应显式形成三层前端结构：

- `ui_shell`
- `application_control`
- `feature_runtime`

目标依赖方向：view 发 intent，control 编排，runtime 持有本功能状态机。

## 本章相关命令

- `Cmd+Shift+P`
- `Cmd+P`
- `Cmd+Shift+K`
- `Cmd+B`

## 本章相关配置

- `ui.theme`
- `ui.sidebar_visible`

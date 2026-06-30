# 11_ui_design/03_mobile.md - 移动端设计 (Mobile UI)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Version`: `0.0.1`
- `Last Review`: `2026-06-28`
- `Counterpart Feature`: `docs/features/08_ui_design_03_mobile.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/05_ui.md`, `docs/acceptance-cases/13_ui_mobile_chat_regression.md`, `docs/acceptance-cases/17_mobile_surface_switcher.md`
- `Primary Code Areas`: `apps/web/src/components/mobile_layout/`, `apps/web/src/components/`, `apps/mobile/`

本章定义 Mobile content-first 适配策略。规范性用语继承 `01_terminology.md`。

> **Current Native Boundary**：Mobile native 是与 Web/Docker 等价的 peer 外壳，支持 `LocalBackend` 与 `RemoteBrowser` 两种互斥模式；壳层本身不拥有业务 authority。
> **Post-Gate Target**：Mobile 端目标采用 **Tauri v2 Mobile packaging** 外壳，共享 Web 前端；Android/Mobile `LocalBackend` 默认启动 embedded loopback full peer service，写入仍必须经 server/core writer gate。`RemoteBrowser` 只作为 HTTPS 远端 Web 壳层。

> **Web 映射**：当 Web 端 $W_{view} \le 768px$ 时，界面 **MUST** 遵循本章 Mobile 规范。

## 1. 原生适配器边界 {#mobile-current-native-boundary}

*   Web 端小屏视口 **MUST** 映射到 Mobile 交互规范。
*   Mobile native adapter 第一阶段只允许承担：选择 shell 模式、启动或绑定本机受控 service endpoint、注入 service endpoint/session、报告 readiness/offline 状态、转发前后台、安全区域与软键盘等有限平台事件，或在 `RemoteBrowser` 中导航到远端 HTTPS origin。
*   默认构建 **MUST** 保持 no-Tauri Mobile skeleton；`tauri` / `tauri-build` dependency 只能作为 `apps/mobile` 的 optional dependency 挂在 `native-packaging` feature 后。
*   `native-packaging` Android/Mobile 默认模式是 `LocalBackend`；Mobile v1 full peer 不使用子进程，而是启动 in-process embedded loopback service，并在 app-private ledger/repo/projection 上自动初始化默认本地 workspace，不依赖 Docker、外部 CLI 或用户手工 init。
*   `RemoteBrowser` **MUST** 显式选择，且只接受远端 `https://host[:port]` origin。URL 不得包含 userinfo、query、fragment 或业务子路径；壳层不得注入本地 endpoint/session bootstrap，不得启动 embedded service。
*   recovery bootstrap 只能表达 `service_offline`、`foreground_reprobe` 与 `session_invalid` 等结构化状态；后台恢复失败 **MUST NOT** 被伪装成普通断网。
*   Mobile native adapter **MUST NOT** 自行定义 Ledger / Projection Workspace authority、schema migration、source-control 语义、同步合并语义或搜索索引语义；这些仍归 core/server。`LocalBackend` 只允许 native 壳启动/绑定本机 embedded full peer service，不授予 shell 直接写 authority。
*   UI readiness **MUST** 等待受控 service 完成 loopback/IPC endpoint 与认证会话绑定后再打开主界面；后台/离线状态不得导致本地编辑进入未声明的半可写状态。

### 1.1 Minimal Native Adapter Contract {#mobile-native-adapter-contract}

Mobile native adapter 与 Desktop 共用 `./02_desktop.md#desktop-native-adapter-contract` 的 authority 边界：native 壳层只负责进程、平台能力与本机 service 绑定，不拥有 ledger/Projection Workspace/source-control/search 的业务真相。

Packaging dependency gate 见 `17_tech_stack.md#native-packaging-dependency-gate`。

### 1.1.1 Mobile Native Shell Modes {#mobile-native-shell-modes}

`NativeShellMode` 的 Mobile 语义如下：

*   `LocalBackend` 是 native-packaging Android/Mobile 默认模式。Mobile 壳层只负责 embedded loopback lifecycle、endpoint/session bootstrap、foreground reprobe、readiness 展示与失败恢复。
*   `LocalBackend` 的本地数据根位于 app-private data root；后端启动前必须由 server/CLI runtime 初始化默认 repo、Projection Locator、workspace identity、`.notegit/` 与 repo-local `.gitignore`。
*   `LocalBackend` 必须复用 server native-session bridge 完成 session handoff，并以 HttpOnly native session cookie 与 `window.__DEVE_NATIVE_BOOTSTRAP` endpoint payload 启动 Web；bootstrap source 不得包含 token、secret 或 auth material。
*   Tauri `main` WebView **MUST** 延迟到 embedded service 完成 probe、native session handoff、bootstrap plugin 与 cookie 注册之后创建；不得先创建无 session/bootstrap 的主 WebView。
*   `RemoteBrowser { https_origin }` 是显式远端模式。壳层只加载远端 Web origin，后续 `/api` 与 `/ws` 均由浏览器同源规则解析；native 壳不提供本机 session cookie、端口、repo bootstrap 或 native bridge。
*   Mobile Settings 必须与 Desktop 共用 native backend preference 语义：默认 `local`；选择 `remote` 时必须先由 Mobile native 侧短超时探测 `<origin>/api/node/role` 并确认结构化 Deve node role，成功后才写入 app-private `native-backend.json`。
*   Mobile `remote` preference 只保存 HTTPS origin，不保存远端凭证、session、token、repo scope 或 writer readiness。启动参数/环境覆盖只用于诊断和脚本启动，不得回写 preference。
*   Mobile Tauri bundle 必须加载 `frontendDist` 资产，并通过 native bootstrap 或 RemoteBrowser 导航决定后端；不得把主 WebView 固定到开发服务 `devUrl = http://127.0.0.1:3001`。
*   Mobile 在 `RemoteBrowser` 失联时沿用普通浏览器锁屏/只读语义；native 锁屏或 Settings 可提供“Use local backend”入口。该入口必须保存 `local` preference、启动 embedded loopback service，并重载 bundled Web shell。
*   从后台恢复时，`LocalBackend` 必须重新 probe session、node role、WS repo handshake 与 current `scope_nonce`；`RemoteBrowser` 的恢复语义等价于浏览器页面恢复，不得伪装本地 authority。

**Adapter inputs**:

*   `profile/config/projection-locator/ledger` 选择必须在 service boot 前完成；Web 运行后 native 层不得直接改写后端路径或 repo scope。
*   `launch_intent` 可表达分享、文件打开、deeplink、通知点击等入口，但必须转为 application command；不得绕过 writer gate。
*   `session_material` 必须绑定到当前 app install 与进程会话；不得放入 URL、Web localStorage、日志或系统剪贴板。
*   `platform_lifecycle` 只允许传递 `foreground/background/suspended/resumed/network-online/network-offline/safe-area/keyboard` 等 shell 事件。

**Adapter outputs**:

*   `NativeEndpointReady { http_base, ws_base, node_role, session_bound }`
*   `NativeServiceOffline { reason, retryable }`
*   `NativeServiceSuspended { reason }`
*   `NativeForegroundReprobe`
*   `NativePlatformEvent { kind }`

**Boot/lifecycle state machine**:

```text
MobileColdStart
  -> ServiceStarting
  -> EndpointBound(http_base, ws_base)
  -> SessionBound
  -> WebShellLoading
  -> RuntimeReady
  -> BackgroundSuspended
  -> ForegroundReprobe
  -> RuntimeReady | ServiceOffline | SessionInvalid
```

`RuntimeReady` 的最小条件与 Desktop 一致：endpoint 可达、`/api/auth/status` 有效、`/api/node/role` 可读、当前 repo 已完成 ws handshake、写入路径满足 `writer_ready(repo_id, scope_nonce)`。`SessionInvalid` 必须进入 `Unauthorized`；后台恢复失败不得伪装成普通断网。

**Endpoint/session injection rules**:

*   Native 壳必须在 Web connection manager 启动前注入 `http_base/ws_base` 与 session 绑定状态；优先使用内存 bridge 或初始 HTML bootstrap。
*   Native 壳可注入只含 `service_state` 的 recovery bootstrap；payload 只能表达 `service_offline`、`foreground_reprobe` 或 `session_invalid`，不得携带 token、session secret、服务失败 reason 或 repo 写权限。
*   `?ws_port=` 只能作为开发期 fallback。mobile production 不得让 Web 端枚举、猜测或扫描本机端口。
*   session 绑定完成前不得打开可写主界面；后台恢复后必须重新 probe session、node role 与 ws repo handshake。

**Offline/background semantics**:

*   `NetworkOffline` 只表示公网不可用；如果 embedded service、session 与 writer gate 仍 ready，本地编辑仍可继续。
*   `BackgroundSuspended` 只允许降低资源占用或暂停远端同步，不得丢弃未 ack 的本地 pending overlay。
*   `ForegroundReprobe` 必须重新执行 `/api/auth/status`、`/api/node/role` 与 repo handshake；旧 `scope_nonce` 不得自动恢复写态。进入 reprobe 时应清空 auth、node-role、repo-handshake、writer-ready 与 scope freshness，直到 fresh readiness 完整通过。
*   `ServiceOffline` 表示本机后端不可达；UI 必须进入恢复/只读状态，不得声称 offline-first 仍可写。

**Forbidden native shortcuts**:

*   native 层不得直接写 ledger/Projection Workspace/source-control/search index。
*   native 层不得直接操作 `.notegit/` 或 `.git/` 来伪造 source-control 成功。
*   safe-area、keyboard、foreground/background、network online/offline 事件不得被解释成业务可写状态。

**Pre-Gate Acceptance Contract**:

*   cold start service bind 失败时显示恢复入口，不进入半可写 UI。
*   session invalid 时进入 `Unauthorized` 并停止普通重连。
*   network offline 但 service/session/writer ready 时，本地编辑继续可用。
*   background/resume 后必须重新握手，stale `scope_nonce` 写入被拒绝。

### 1.2 Embedded Service Supervisor Contract {#mobile-service-supervisor-contract}

Mobile 与 Desktop 共用 `./02_desktop.md#desktop-service-supervisor-contract`
的 supervisor 状态机，但 Mobile 额外保持生命周期约束：

*   `EndpointHealthy` 和 `SessionHandoffReady` 不得绕过 `ForegroundReprobe`；从后台恢复后仍必须重新 probe auth、node role、WS repo handshake 与 current `scope_nonce`。
*   `NetworkOffline` 仍只是公网提示；只有 embedded service health probe 失败才可进入 `ServiceOffline`。
*   `BackgroundSuspended` 不得清空 pending overlay，也不得把旧 supervisor session handoff 直接视为可写。
*   Health probe、retry budget、session handoff failure 分类与 Desktop 一致：bind/probe/process-exit 可预算内 retry，session handoff failure fatal。
*   supervisor 不得写 ledger/Projection Workspace/source-control/search index/`.git`/`.notegit`。

### 1.3 Process Adapter Gate {#mobile-process-adapter-decision}

Mobile process adapter gate 对默认 no-Tauri Mobile skeleton 仍关闭；真实 mobile child-process runtime **MUST NOT** 进入默认 no-Tauri Mobile skeleton。Android/Mobile `LocalBackend` 使用 embedded loopback service，不使用移动端子进程；`RemoteBrowser` 关闭 embedded service。

Gate policy 必须满足：

*   默认 no-Tauri `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY.decision =
    DeferredUntilPackagingGate`
*   默认 no-Tauri `child_process_runtime_enabled = false`
*   `packaging_gate_required = true`
*   native shell 直接 `authority_writes_allowed = false`

Mobile `LocalBackend` policy 必须满足：

*   `decision = LocalBackendDefault`
*   `child_process_runtime_enabled = false`
*   `embedded_service_runtime_enabled = true`
*   `packaging_gate_required = true`
*   native shell 直接 `authority_writes_allowed = false`

真实 mobile embedded service runtime 必须位于 Mobile native adapter 的 `native-packaging` feature 后，只做 loopback endpoint、session handoff、foreground reprobe 与 runtime readiness wiring；不得绕过 writer-ready 或 repo scope gate。

### 1.4 Mobile Packaging Scaffold {#mobile-packaging-scaffold}

Mobile packaging scaffold 只描述移动壳层的 dependency spike 与 post-gate 目标能力，**MUST NOT** 被解释为 release ready、process runtime 或 native authority 已显式启用。Android shell-only package execution 由 §1.6 单独门禁；iOS shell-only package execution 由 §1.7 单独门禁：

*   dependency batch: `tauri` runtime crate + `tauri-build` build crate。
*   packaging capability 只覆盖移动壳层能力：WebView shell、permission bridge、share sheet、
    deeplink、file picker、push notification、store package。
*   packaging dependency spike 不得获得 ledger/Projection Workspace/source-control/search index/`.git`/`.notegit`
    authority；这些业务真相仍只归 core/server。
*   lifecycle correctness 仍由 no-packaging mobile skeleton tests 保证：background/resume 后必须
    fresh reprobe auth、node role、WS repo handshake 与 current `scope_nonce`，packaging 不得绕过。
*   `scripts/check-native-track-boundary.sh` 必须继续阻止 packaging dependency 或 import
    泄漏到 workspace root、core、cli、web 或 native 默认构建。

### 1.5 Mobile Packaging Dependency Gate {#mobile-packaging-dependency-gate-decision}

Mobile packaging dependency gate 已进入 Mobile dependency spike；真实 `tauri` / `tauri-build`
dependency 只允许作为 `apps/mobile` 的 optional dependency，并且必须挂在 `native-packaging`
feature 后。默认 workspace 构建仍 **MUST** 保持 no-Tauri。

Gate policy 必须满足：

*   `CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY.decision =
    DesktopAndMobileDependencySpikeOpen`
*   `desktop_tauri_dependencies_allowed = true`
*   `mobile_tauri_dependencies_allowed = true`
*   `default_build_remains_no_tauri = true`
*   `native_feature_gate_required = true`
*   `authority_writes_allowed = false`

当前 gate 分为四层：

*   Mobile dependency spike 已打开：`tauri` / `tauri-build` 只能作为 `apps/mobile`
    的 optional dependency 存在。
*   Android shell-only package execution 可按 §1.6 单独打开。
*   iOS shell-only package execution 可按 §1.7 单独打开。
*   Mobile child-process supervision 与 native shell direct authority write path 仍未打开；Android/Mobile `LocalBackend` 只打开 embedded loopback service 承载。

Foreground reprobe、writer-ready 与 repo scope gate **MUST NOT** 被 native runtime 绕过。

### 1.6 Android Shell-only Package Execution Gate {#mobile-android-shell-package-execution-gate}

Android shell-only package execution gate 只允许把 Mobile WebView 壳层推进到 Android target-host package execution；它不是 Mobile release ready，也不是 process adapter gate。

Gate policy 必须满足：

*   Android required preflight **MUST** 先通过。
*   Android project generation 与 package build **MAY** 只在 `apps/mobile` 的 `native-packaging`
    feature 与显式 target-host script 下执行。
*   Android package build **MUST** 声明 WebView shell、manifest、permission bridge、
    deeplink/share/file/store package 等壳层能力；`LocalBackend` 只通过 embedded loopback service 承载本机 full peer，不得使用子进程。
*   Android package build **MUST NOT** 启动、持有、重启后端子进程。
*   Android package build **MUST NOT** 写 ledger/Projection Workspace/source-control/search index/`.git`/`.notegit`。
*   Android package build **MUST NOT** 绕过 foreground reprobe、session handoff、node-role
    probe、repo handshake、writer-ready 或 current `scope_nonce`。
*   Android package execution 成功 **MUST NOT** 声明 iOS ready、Desktop ready、release ready
    或 process runtime ready。

### 1.7 iOS Shell-only Package Execution Gate {#mobile-ios-shell-package-execution-gate}

iOS shell-only package execution gate 只允许把 Mobile WebView 壳层推进到 iOS target-host package execution；它不是 Mobile release ready，也不是 process adapter gate。

Gate policy 必须满足：

*   iOS required preflight **MUST** 先在 macOS target host 通过。
*   iOS project generation 与 package build **MAY** 只在 `apps/mobile` 的 `native-packaging`
    feature 与显式 target-host script 下执行。
*   iOS package build **MUST** 只声明 WebView shell、manifest、permission bridge、
    deeplink/share/file/store package 等壳层能力。
*   iOS package build **MUST NOT** 启动、持有、重启后端子进程。
*   iOS package build **MUST NOT** 写 ledger/Projection Workspace/source-control/search index/`.git`/`.notegit`。
*   iOS package build **MUST NOT** 绕过 foreground reprobe、session handoff、node-role
    probe、repo handshake、writer-ready 或 current `scope_nonce`。
*   iOS package execution 成功 **MUST NOT** 声明 Android ready、Desktop ready、release ready
    或 process runtime ready。

## 2. Responsive Architecture {#mobile-responsive-layout}

### 2.1 布局状态机 (Layout State Machine)
系统布局 $L$ 根据视口宽度 $W_{view}$ 在两种状态间切换：

*   **Desktop State**: $W_{view} > 768px \implies$ Grid Layout.
*   **Mobile State**: $W_{view} \le 768px \implies$ Stack Layout.

### 2.2 视口配置 (Viewport Configuration)
HTML Header **MUST** 适配刘海屏并禁止 iOS 自动缩放：

```html
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
```

并且所有固定定位元素 **MUST** 使用 CSS `env()` 适配安全区域：
*   `padding-top: env(safe-area-inset-top)`
*   `padding-bottom: env(safe-area-inset-bottom)`

## 3. Interaction Design {#mobile-interaction-design}

### 3.1 导航策略 (Navigation)
移动端移除常驻侧边栏，改为 **Drawer (抽屉)** 模式。

*   **Left Drawer (Sidebar)**:
*   **Trigger**: 左上角汉堡菜单 (`≡`) 或 **屏幕左边缘右滑 (Edge Swipe)**。
*   **Visual**: 覆盖在编辑器之上，背景带有半透明 Backdrop (`z-index: 100`).
*   **Right Drawer (Outline)**:
*   **Trigger**: 编辑器内容区右上角的 `Toggle Outline` 浮动图标（非 Top Bar）或 **屏幕右边缘左滑**。

### 3.2 面板宽度策略 (Panel Width Policy)

*   **Resizable Handles**: 移动端 **SHOULD NOT** 显示左右拉伸手柄。
*   **Persistence**: 仍可读取已保存的桌面宽度，但移动端不提供调整入口。
*   **Outer Gutter**: 移动端 **SHOULD NOT** 提供外边距拖拽。

### 3.3 虚拟辅助键盘栏 (Mobile Toolbar)
系统 **MUST** 在软键盘上方渲染 Markdown Accessory View。

**Key Layout (Visual Representation)**:

| `↩` Undo | `↪` Redo | `⇥` Tab | `H`ead | `•` List | `☑` Task | `B`old | `I`talic | `<>` Code |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| Cmd+Z | Cmd+Shift+Z | Indent | `#` | `-` | `[ ]` | `**` | `_` | \` |

**Technical Constraint**:
必须使用 `visualViewport` API 监听键盘高度变化，动态调整 Toolbar 的 `bottom` 偏移量，防止被键盘遮挡。
撤销与重做按钮 **MUST** 与其它写动作共用 repo writer gate；只读、握手中、快照加载中、writer 未就绪或 scope switching 时不得触发编辑器 history action。
撤销与重做属于高频恢复操作，**MUST** 保持在移动工具栏前段，390px 宽度下无需横向滚动即可看到。
Toolbar **SHOULD** 仅在软键盘可见时显示；软键盘弹出时底部状态栏可暂时让位以优先输入。

### 3.4 手势系统 (Gesture System)
仅支持轻量级 Edge Swipe，参数定义如下：
*   $Zone_{edge} = 20px$ (从屏幕边缘起算的响应区)。
*   $Threshold_{swipe} = 50px$ (触发滑动的最小距离)。
*   **Interactive Safety**: Edge Swipe **MUST NOT** 抢占靠边可交互控件的真实点击，例如 `File tree`、`Toggle Outline` 等按钮。

## 4. Visual Adaptations

### 4.1 布局约束
*   **Diff View**: 移动端 **MUST NOT** 使用左右并排 (Side-by-Side) 对比，而应强制回退到 **Unified View** (单列混合)。
*   **Font Size**: 默认字号 **SHOULD** 设为 `16px` 以避免 iOS Safari 输入时强制放大页面。

### 4.2 只读模式指示器 (Spectator Indicator)
在 Spectator Mode 下，顶部导航栏下方 **MUST** 插入一条醒目的橙色横幅 (`Height: 24px`)，提示 "Read-Only Mode"。

## 5. Mobile UI Layout

### 5.1 结构层级 (Hierarchy)
*   **Top App Bar**: 固定顶部，包含导航与核心操作。
*   **Content Stack**: 单列内容区，默认全屏编辑器。
*   **Bottom Bar**: 与 Desktop 功能对齐（Branch/连接状态/加载状态/历史条/统计），移动端允许多行折叠布局。
    *   默认折叠态 **MUST** 仅显示一行：`Branch / Ready / Words / Lines / Col`。
    *   通过右侧箭头按钮展开详情；再次点击或点击状态栏外区域自动收起。
    *   折叠态信息 **SHOULD** 无需横向滚动。
    *   分支名在折叠态 **SHOULD** 自动截断，避免挤压状态与统计信息。

### 5.2 顶部导航栏 (Top App Bar)
*   **Left**: Hamburger Menu (`≡`) 打开 Sidebar Drawer。
    *   该入口按钮文案/可访问性语义建议统一为 `File tree`。
*   **Center**: 文档标题/仓库名（省略溢出）。
*   **Right**: Home / Open / Command（与 Desktop 顶栏语义一致）。

### 5.3 Drawer 规范 (Side Drawers)
*   **Sidebar Drawer**:
    *   内容：文件树、快速操作、新建。
    *   关闭按钮建议使用 `X` 图标而非文本 `Close`，以符合移动端通用习惯。
    *   行为：点击文件后自动收起。
    *   `More(...)` 菜单 **MUST** 复用桌面端语义：整行点击切换视图，`Pin/Unpin` 仅修改固定状态，不得伪装成“点击无反应”。
    *   Source Control tab **MUST** 复用共享 Source Control read surface 与 read gate；移动端不得把正常 `Staged Changes` / `Changes` / `Confirmed Ledger Changes` 视图退化成 `git status` CLI-only Git bridge notice。
*   **Outline Drawer**:
    *   内容：标题结构、大纲条目。
    *   行为：点击条目后自动收起并滚动定位。

### 5.4 编辑器区 (Editor)
*   **Mode**: 单列布局，支持全屏编辑。
*   **Diff**: 强制 Unified View。
*   **Selection**: 单指选区，长按弹出操作菜单。

### 5.4.1 Mobile Surface Switcher {#mobile-surface-switcher}

移动端必须复用 `index.md#editor-group-tabstrip` 定义的 document/diff tab identity 与
view-local lifecycle，但不得直接复制桌面横向 tabstrip。

*   顶部当前 surface 胶囊 **MUST** 位于 sync banner 与 content stack 之间，显示当前
    document/diff 名称、surface 类型与已打开数量。
*   点击胶囊 **MUST** 打开底部 sheet；sheet 分组显示 Documents 与 Diffs，并提供选择、
    关闭与 active 标记。
*   sheet 行与关闭按钮 **MUST** 满足移动端 44px touch target。
*   选择 document 必须复用 guarded document navigation；选择 diff 只能恢复已有
    view-local `DiffSession`，不得触发新的 diff 计算。
*   关闭 active diff 只关闭 diff surface，不得修改 staged、pending 或 commit state。
*   repo / branch / scope 切换、drawer 打开、选择条目或关闭当前 surface 时，sheet
    **MUST** 自动收起。
*   sheet 打开时 **MUST** 避免 AI Chat、辅助键盘栏与 Bottom Bar 遮挡。

### 5.5 快捷入口 (Quick Actions)
*   **Search**: 打开 Quick Open / Command Palette（移动端应为底部抽屉）。
*   **Sync**: 可在 More 菜单中触发。

## 6. Key Flows

### 6.1 打开文档
1. 点击 Hamburger -> 打开 Sidebar Drawer。
2. 选择文档 -> Drawer 自动收起 -> Editor 渲染。

### 6.2 查看大纲
1. 点击编辑器内容区右上角 `Toggle Outline` 图标 -> 打开右侧 Drawer。
2. 点击条目 -> Drawer 自动收起 -> Editor 滚动定位。

### 6.3 搜索/命令
1. 点击 Search -> Top Sheet 自上而下展开。
2. 选择结果 -> 自动关闭并跳转。
3. 关闭手势以顶部拖拽上滑为主（避免与结果列表滚动冲突）。

## 7. Performance & Size
*   **Target**: 首屏渲染 < 1s，输入延迟 < 16ms。
*   **Memory**: 低端设备 **MUST** 平稳运行。
*   **Dependency**: 移动端 **MUST** 避免重型 UI 框架。

### 7.1 Visual Reference (Yuque-Inspired)
*   **Tone**: 轻盈、克制、阅读优先。
*   **Layout**: 卡片化信息层级，内容区留白适中。
*   **Typography**: 标题略微加重，正文中性字重，行高舒适。
*   **Surface**: 浅色背景 + 轻阴影，强调内容层次而非装饰。
*   **Interaction**: 抽屉与底部面板动效柔和，避免夸张动画。

## 8. Related Configuration

*   `ui.mobile.font_size`: 移动端专用字号 (Default: 16).
*   `ui.mobile.toolbar_visible`: 是否显示辅助键盘栏 (Default: true).

## 9. Post-Gate Implementation Target

### 9.1 移动端 UI 方案

本节是 post-gate normative target：只有 `native-packaging` 与 process adapter gate 显式打开后，以下规则才进入验收；默认 no-Tauri Mobile skeleton 仍以 §1 为准。

*   **Rule**: Mobile post-gate 采用 **Tauri v2 Mobile** 作为外壳（WKWebView / Android WebView），前端代码与 Web 端共享，配合原生层访问摄像头/文件系统/推送等系统 API。
*   **Consistency**: 交互与布局规则 **MUST** 与本章一致，行为不以 Web 端为准。

### 9.2 Common Post-Gate Contract

Mobile post-gate **MUST** 服从 `./index.md#native-post-gate-common-contract`。

### 9.3 Mobile Deltas

*   App 进入后台时 **SHOULD** 降低服务资源占用；恢复前台时自动唤醒。
*   post-gate 无公网时 **MUST** 提供本地读写能力；完整后台/索引/同步能力仍受 platform lifecycle、profile、feature 与资源预算约束。
*   网络恢复后 **SHOULD** 执行增量同步；失败不得影响本地编辑。
*   体积 **MUST** 可控，避免引入重型依赖。
*   输入延迟与滚动流畅性 **MUST** 优先保障。

## 10. Web Mobile Alignment Contract

在 `W_view <= 768px` 的 Web 视口中，Web shell **MUST** 按本章 Mobile 规范降级，而不是继续套用 Desktop 交互。

必须保持的对齐点：

*   移动端 **MUST NOT** 显示桌面左右拉伸手柄或外边距拖拽入口。
*   主编辑区默认字号 **SHOULD** 不低于 `16px`，避免 iOS 输入焦点自动缩放。
*   左右 Drawer、Top Sheet、Bottom Sheet、Outline、Search Result 与 Source Control 面板 **MUST** 遵守同一套 touch target、focus 与 selected/active 语义。
*   Bottom Sheet 手势关闭 **MUST** 具备阈值、防抖与滚动冲突判定；轻微位移不得误关闭。
*   边缘滑动 **MUST NOT** 抢占靠边真实控件点击。
*   `More(...)` 菜单 **MUST** 复用桌面端语义：整行点击切换视图，`Pin/Unpin` 只改变固定状态。
*   移动端 Diff **MUST** 使用 Unified View，并避免 AI Chat、辅助键盘栏或抽屉层级遮挡 diff 操作。
*   移动端 AI Chat **SHOULD** 以页面级或全屏交互呈现，避免半屏手势与编辑器/键盘层级冲突。

## 11. Mobile SHOULD Boundary

以下 SHOULD 只定义责任边界；Web 映射不得据此扩展为原生能力声明：

*   Resizable Handles：移动端 **SHOULD NOT** 显示左右拉伸手柄。
*   Outer Gutter：移动端 **SHOULD NOT** 提供外边距拖拽。
*   Font Size：移动端编辑器默认字号 **SHOULD** 设为 `16px` 或更高。
*   Background Resource：原生 Mobile App 后台时服务 **SHOULD** 降低资源占用；Web 映射只能表达前台视口行为。
*   Firewall：移动端内嵌服务 **SHOULD** 显式阻断非回环访问。
*   Export：单文档/全量导出属于原生端或后端服务能力，**SHOULD** 由对应章节定义实现路径。
*   Audit：关键操作日志属于 core/service 审计链路，**SHOULD** 避免塞入移动 UI 层。
*   Recovery Drill：恢复演练属于 release/ops 能力，**SHOULD** 由发布与运维流程承接。

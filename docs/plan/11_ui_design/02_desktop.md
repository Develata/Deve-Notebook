# 11_ui_design/02_desktop.md - 桌面端设计 (Desktop UI)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Version`: `0.0.1`
- `Last Review`: `2026-06-19`
- `Counterpart Feature`: `docs/features/08_ui_design_02_desktop.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/05_ui.md`
- `Primary Code Areas`: `apps/web/src/components/`, `apps/web/src/hooks/use_core/`, `apps/desktop/`

本章定义 Desktop 布局与交互。规范性用语继承 `01_terminology.md`。

> **Current Native Boundary**：Desktop native 是与 Web/Docker 等价的 peer 外壳，支持 `LocalBackend` 与 `RemoteBrowser` 两种互斥模式；壳层本身不拥有业务 authority。
> **Post-Gate Target**：Desktop 端目标采用 **Tauri v2 native packaging** 外壳，共享 Web 前端；native-packaging 默认进入 `LocalBackend`，启动受控本机 full peer service 并经 server/core writer gate 写入。`RemoteBrowser` 只作为 HTTPS 远端 Web 壳层。

> **Web 映射**：当 Web 端 $W_{view} > 768px$ 时，界面 **MUST** 遵循本章 Desktop 规范。

## 1. 原生适配器边界 {#desktop-current-native-boundary}

*   Web 端大屏视口 **MUST** 映射到 Desktop 交互规范。
*   Native adapter 第一阶段只允许承担：选择 shell 模式、启动或绑定本机受控 service endpoint、注入 service endpoint/session、报告 readiness/offline 状态、转发有限平台事件，或在 `RemoteBrowser` 中导航到远端 HTTPS origin。
*   默认构建 **MUST** 保持 no-Tauri skeleton；真实 `tauri` / `tauri-build` dependency 只能在 `native-packaging` feature 与独立 gate 打开后引入。
*   `native-packaging` Desktop 默认模式是 `LocalBackend`；它 **MUST** 启动、持有或重启受控后端子进程，并在 app-private ledger/repo/projection 上自动初始化默认本地 workspace，不依赖 Docker、外部 CLI 或用户手工 init。
*   `RemoteBrowser` **MUST** 显式选择，且只接受远端 `https://host[:port]` origin。URL 不得包含 userinfo、query、fragment 或业务子路径；壳层不得注入本地 endpoint/session bootstrap，不得启动本机 service。
*   recovery bootstrap 只能表达 `service_offline`、`foreground_reprobe` 与 `session_invalid` 等结构化状态；无效 endpoint 或 session-pending **MUST NOT** 退化为端口扫描。
*   Native adapter **MUST NOT** 重新定义 Ledger / Projection Workspace authority、schema migration、source-control 语义或搜索索引语义；这些仍归 core/server。`LocalBackend` 只允许 native 壳启动/绑定本机 server full peer，不授予 shell 直接写 authority。
*   UI readiness **MUST** 等待受控 service 完成 loopback/IPC endpoint 与认证会话绑定后再打开主界面；失败时显示恢复入口而不是进入半可写状态。

### 1.1 Minimal Native Adapter Contract {#desktop-native-adapter-contract}

Desktop native adapter 是进程与平台壳层，不是业务 authority；第一阶段把 Web shell 绑定到本机受控 service，或以 RemoteBrowser 方式导航到远端 HTTPS origin，并向 Web/application control 交付结构化 runtime 状态。

### 1.1.1 Desktop Native Shell Modes {#desktop-native-shell-modes}

`NativeShellMode` 的 Desktop 语义如下：

*   `LocalBackend` 是 native-packaging 默认模式。Desktop 壳层只负责 sidecar 生命周期、loopback endpoint、native session handoff、health probe、restart budget、bootstrap/recovery 注入与窗口/菜单/托盘事件。
*   `LocalBackend` 的本地数据根位于 app-private data root（诊断覆盖入口：`DEVE_DESKTOP_DATA_DIR`）；不得把 shell 启动目录当作默认数据根。后端启动前必须由 server/CLI runtime 初始化默认 repo、Projection Locator、workspace identity、`.notegit/` 与 repo-local `.gitignore`。
*   `RemoteBrowser { https_origin }` 是显式远端模式，Desktop 可通过启动参数 `--remote-url https://host[:port]` 或诊断/脚本环境变量 `DEVE_NATIVE_REMOTE_URL` 选择。壳层只加载远端 Web origin，后续 `/api` 与 `/ws` 均由浏览器同源规则解析；native 壳不提供本机 session cookie、端口、repo bootstrap 或 native bridge。
*   模式切换不得复用旧 session、旧 endpoint 或旧 `scope_nonce`。从 `RemoteBrowser` 回到 `LocalBackend` 必须重新启动本机 service 并完成完整 session handoff。

Packaging dependency gate 见 `17_tech_stack.md#native-packaging-dependency-gate`。

### 1.2 Desktop Packaging Scaffold {#desktop-packaging-scaffold}

Desktop packaging scaffold 只描述桌面壳层的 post-gate 目标能力，**MUST NOT** 被解释为 packaging gate 已显式启用：

*   dependency batch: `tauri` + `tauri-build`，只能落在 Desktop native adapter 的 feature scope。
*   capabilities: window shell、menu bar、system tray、installer、auto-update。
*   forbidden authorities: ledger、Projection Workspace、source-control、search index、`.git` mirror、
    `.notegit`。
*   no-packaging skeleton tests 仍是 endpoint/session/bootstrap/readiness correctness 的
    authority；packaging acceptance 不得替代这些测试。

除 Desktop dependency spike 已允许范围外，`scripts/check-native-track-boundary.sh` 必须继续阻止
Cargo dependency/import 泄漏。

### 1.3 Desktop Packaging Dependency Gate {#desktop-packaging-dependency-gate-decision}

Desktop packaging dependency gate 已进入 Desktop dependency spike；真实 `tauri` / `tauri-build`
dependency 只允许作为 `apps/desktop` 的 optional dependency，并且必须挂在 `native-packaging`
feature 后。默认 workspace 构建仍 **MUST** 保持 no-Tauri。

Gate policy 必须满足：

*   `CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY.decision =
    DesktopAndMobileDependencySpikeOpen`
*   `desktop_tauri_dependencies_allowed = true`
*   `mobile_tauri_dependencies_allowed = true`
*   `default_build_remains_no_tauri = true`
*   `native_feature_gate_required = true`
*   `authority_writes_allowed = false`

当前 gate 允许 Desktop 与 Mobile dependency spike：packaging 默认不获得 shell direct ledger/Projection Workspace/source-control/search/`.git`/`.notegit` authority，不声明 macOS/Windows/Android/iOS release ready。Desktop `LocalBackend` 后端子进程只能作为本机 full peer 承载路径打开，shell 仍不得直接写 authority。

### 1.4 Embedded Service Supervisor Contract {#desktop-service-supervisor-contract}

Desktop native supervisor contract 固定 supervisor 状态机与 failure 分类，防止 service readiness 与业务写权限混用：

```text
Idle
  -> Starting
  -> EndpointHealthy
  -> SessionHandoffReady
  -> Restarting | Offline
```

**Supervisor rules**:

*   `EndpointHealthy` 只表示 loopback endpoint 可达且 `/api/node/role` 可读；它不代表 Web 已可写。
*   `SessionHandoffReady` 必须在 `EndpointHealthy` 后发生，并且要求 session 已绑定；session handoff 失败是 fatal offline，不自动重试。
*   `BindFailed`、`HealthProbeFailed`、`ProcessExited` 可在 retry budget 内进入 `Restarting`；超过预算后进入 `Offline`。
*   `SpawnFailed` 与 `SessionHandoffFailed` 默认不可重试，必须进入 `Offline`。
*   supervisor 的 `offline.reason` 是 native 内部诊断；recovery bootstrap 仍不得把 reason、token、secret 或 repo 写权限暴露给 Web。
*   supervisor 不得写 ledger/Projection Workspace/source-control/search index/`.git`/`.notegit`。

**Adapter inputs**:

*   `profile/config/projection-locator/ledger` 选择必须在 service boot 前完成，并传入后端启动参数；native 层不得在 Web 运行后直接改写这些路径。
*   `launch_intent` 只表示打开仓库、文档或 deeplink 的意图，必须转为普通 application command；不得绕过 repo scope gate 直接写 ledger。
*   `session_material` 必须是进程内或同站 cookie 绑定的短生命周期本机会话材料；不得放入 URL、localStorage、日志或 crash report。

**Adapter outputs**:

*   `NativeEndpointReady { http_base, ws_base, node_role, session_bound }`
*   `NativeServiceOffline { reason, retryable }`
*   `NativeServiceRestarting { attempt }`
*   `NativePlatformEvent { kind }`，其中 `kind` 仅允许表达窗口焦点、主题、系统网络 online/offline、关闭/后台驻留请求等 shell 事件。

**Boot state machine**:

```text
NativeColdStart
  -> ServiceStarting
  -> EndpointBound(http_base, ws_base)
  -> SessionBound
  -> WebShellLoading
  -> RuntimeReady
  -> ServiceRestarting | ServiceOffline | SessionInvalid
```

`RuntimeReady` 的最小条件：endpoint 可达、`/api/auth/status` 有效、`/api/node/role` 可读、当前 repo 已完成 ws handshake、写入路径满足 `writer_ready(repo_id, scope_nonce)`。`SessionInvalid` 必须进入 `Unauthorized`，不得包装成普通 `Disconnected`。

**Endpoint/session injection rules**:

*   Native 壳必须在 Web connection manager 启动前注入 `http_base/ws_base` 与 session 绑定状态；优先使用内存 bridge 或初始 HTML bootstrap。
*   Native 壳可注入只含 `service_state` 的 recovery bootstrap；payload 只能表达 `service_offline`、`foreground_reprobe` 或 `session_invalid`，不得携带 token、session secret、服务失败 reason 或 repo 写权限。
*   Desktop Tauri manifest 不得把 native Web shell 固定到后端端口（例如 `devUrl = http://127.0.0.1:3001`）。Desktop shell 必须加载 bundled/frontendDist 资产，并只通过 native bootstrap 获取 LocalBackend endpoint 或 RemoteBrowser origin。
*   `?ws_port=` 只能作为开发期 fallback。native production 不得让 Web 端枚举、猜测或扫描本机端口。
*   session 绑定完成前 Web shell 不得显示可写主界面；过期 session 必须走 `08_auth.md#unauthorized-disconnected-ui`。

**Offline/readiness semantics**:

*   `NetworkOffline` 只表示公网不可用；如果 embedded service、session 与 writer gate 仍 ready，Desktop 本地编辑仍可继续。
*   `ServiceOffline` 表示本机后端不可达；UI 必须进入恢复/只读状态，不得假装仍有本地 authority。
*   App 从后台/驻留状态恢复时必须重新 probe `/api/auth/status`、`/api/node/role`，并重新确认 ws repo handshake；旧 `scope_nonce` 不得自动恢复写态。
*   `RuntimeReady` 只有在 endpoint/auth/node-role/repo-handshake/writer-ready/current-scope 全部满足时成立；`Foreground` 或 `Resumed` 事件会进入 `ForegroundReprobe`，直到 fresh readiness 完整通过。

**Forbidden native shortcuts**:

*   native 层不得直接写 ledger/Projection Workspace/source-control/search index。
*   native 层不得直接操作 `.notegit/` 或 `.git/` 来伪造 source-control 成功。
*   native 层不得把平台 online/offline、窗口焦点或 Tauri lifecycle 事件解释成业务可写状态。

**Pre-Gate Acceptance Contract**:

*   service bind 失败时显示恢复入口，不进入半可写 UI。
*   session invalid 时进入 `Unauthorized` 并停止普通重连。
*   network offline 但 service/session/writer ready 时，本地编辑继续可用。
*   resume/restart 后必须重新握手，stale `scope_nonce` 写入被拒绝。

### 1.5 Process Adapter Gate {#desktop-process-adapter-decision}

Desktop process adapter gate 对默认 no-Tauri skeleton 仍关闭；真实 desktop child-process runtime **MUST NOT** 进入默认 no-Tauri skeleton。`native-packaging` Desktop 的 `LocalBackend` 默认打开受控 local service runtime；`RemoteBrowser` 关闭本机 runtime。

Gate policy 必须满足：

*   默认 no-Tauri `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY.decision =
    DeferredUntilPackagingGate`
*   默认 no-Tauri `child_process_runtime_enabled = false`
*   `packaging_gate_required = true`
*   native shell 直接 `authority_writes_allowed = false`

Desktop `LocalBackend` policy 必须满足：

*   `decision = LocalBackendDefault`
*   `child_process_runtime_enabled = true`
*   `packaging_gate_required = true`
*   native shell 直接 `authority_writes_allowed = false`

真实 process adapter 打开时必须满足：

*   仍只落在 Desktop native adapter 的 `native-packaging` feature 后，不得进入 workspace
    root、core、cli 或 web 默认构建。
*   只允许做受控 child-process spawn、health-probe、session-handoff、restart-budget
    wiring；native shell 不得直接写 ledger/Projection Workspace/source-control/search/`.git`/`.notegit`。
*   所有可写 UI 仍由 server/core 的 repo scope、writer-ready 与 `scope_nonce` 决定；
    process running 不等于 writable。
*   local service 只能监听 loopback；端口、session material 与 bootstrap secret 不得进入 URL、日志或 Web localStorage。

## 2. The Cockpit Concept (布局哲学)

桌面端设计遵循 **"Information Stratification" (信息分层)** 原则，将界面划分为不同关注度的区域。

*   **L1 (Focus)**: 编辑区 (Editor)。绝对中心，无干扰。
*   **L2 (Context)**: 侧边栏 (Sidebar)。提供导航上下文 (Explorer, Outline)。
*   **L3 (Meta)**: 状态栏 (Status Bar)。提供系统元数据 (Git Branch, Sync Status)。
*   **L4 (Floating)**: 悬浮层 (Overlays)。按需出现的命令入口。

## 3. Dynamic Grid System

### 3.1 布局定义 (Layout Definition)
桌面外层采用 3 区域动态 splitter。形式化定义如下：

$$ Splitter = [Region_{sidebar}, Region_{display\_editor}, Region_{chat}] $$

*   **Constraint**: $Region_{display\_editor}$ 由左右两条分界线共同决定；宽度可为 `0px`，但分界线不得因此消失。
*   **Internal Surfaces**: diff old/new、editor、outline 等仍属于 `Region_{display_editor}` 内部 surface；它们不得反向改写外层 splitter 状态。
*   **CSS Implementation**:
    ```css
    display: grid;
    grid-template-columns: var(--w-sidebar) var(--divider) minmax(0, 1fr) var(--divider) var(--w-chat);
    ```

### 3.2 布局可视化 (Visualization)

**Main Workbench Structure**:

| Layer      | Sidebar Region | Display / Editor Region | Chat Region |
| :--------- | :------------- | :---------------------- | :---------- |
| **Top**    | `[Explorer]`   | Editor / Diff / Outline | `AI Chat`   |
| **Body**   | File Tree      | Writable / Read-only surfaces | Chat Log |
| **Resize** | right edge divider | left and right edge dividers | left edge divider |

### 3.3 组件规范 (Component Specs)

*   **Primary Sidebar (Col 1)**:
    *   **Behavior**: **MUST** 支持拖拽调整宽度，最小可折叠到 `0px`。
    *   **Persistence**: **MUST** 记住用户设置。
    *   **Divider**: 宽度为 `0px` 时，Sidebar 与 Display / Editor 之间的分界线仍必须可见并可拖回。
*   **Display / Editor Region**:
    *   **Behavior**: **MUST** 由左右两条分界线共同调整宽度，最小可折叠到 `0px`。
    *   **State**: 折叠只改变 UI layout size，不得清理当前文档、diff session、pending navigation 或 source-control 状态。
*   **AI Chat Region (Right Panel)**:
    *   **Behavior**: **MUST** 支持拖拽调整宽度，最小可折叠到 `0px`。
    *   **Persistence**: **MUST** 记住用户设置。
    *   **Divider**: 普通拖拽折叠到 `0px` 时，Chat 左侧分界线仍必须可见并可拖回。
    *   **Settings Visibility**: Settings 隐藏 AI Chat 时，Chat 区域与 Chat 左侧分界线必须同时不渲染；该行为不得复用 `0px` 折叠状态。
*   **Editor Area (Col 2 & 3)**:
    *   **Single Mode**: $Width(Col_2) = 0$。
    *   **Diff Mode**: $Width(Col_2) = 50\%$。
    *   **Scroll Sync**: 当滚动 Col 3 时，Col 2 必须根据文档高度比例同步滚动。
*   **Unified Search Modal (The Brain)**:
    *   **Definition**: 全局统一的输入入口 $I$。
    *   **Modes**:
        *   `Command`: Prefix `>` (e.g., `>Toggle Sidebar`).
        *   `File`: No Prefix (e.g., `src/main.rs`).
        *   `Branch`: Prefix `@` (e.g., `@feature/xyz`).

## 4. Source Control UI

Source Control view 的详细信息架构、resource group、row action、menu 与 VS Code-like reference policy 由 `12_source_control_ui.md` 定义。本节只保留 Desktop shell 下的可见性与布局约束。

### 4.1 视图结构 (View Structure)
定义源代码管理视图 $V_{sc}$ 为三个集合的并集：
$$ V_{sc} = S_{staged} \cup S_{unstaged} \cup H_{commits} $$

*   **Staged ($S_{staged}$)**: 已暂存的文件集合。支持 `Unstage All`。
*   **Unstaged ($S_{unstaged}$)**: 工作区的脏文件集合。支持 `Stage All` / `Discard All`。

### 4.2 变更状态可视化
每个变更项 $Item \in V_{sc}$ **MUST** 使用语义化颜色标记状态：

*   **Modified ($M$)**: Orange (`var(--color-modified)`).
*   **Added ($A$)**: Green (`var(--color-added)`).
*   **Deleted ($D$)**: Red (`var(--color-deleted)`).

## 5. Related Commands

*   `view.layout.toggle_sidebar`: 切换侧边栏可见性。
*   `view.layout.toggle_diff`: 切换 Diff/Editor 模式。
*   `source_control.stage_all`: 暂存所有更改。

## 6. Post-Gate Implementation Target

### 6.1 跨平台 UI 方案

本节是 post-gate normative target：只有 `native-packaging` 与 process adapter gate 显式打开后，以下规则才进入验收；pre-gate 边界以 §1 为准。

*   **Rule**: Desktop post-gate 采用 **Tauri v2 (WebView)** 作为跨平台外壳，前端代码与 Web 端共享。
*   **Consistency**: 交互与布局规则 **MUST** 与本章一致。
*   **Note**: "原生 UI" 在此指用户体验层面（窗口管理、菜单栏、系统托盘等），而非技术实现层面。

### 6.2 Common Post-Gate Contract

Desktop post-gate **MUST** 服从 `./index.md#native-post-gate-common-contract`。

### 6.3 Desktop Deltas

*   关闭主窗口 **SHOULD** 提供安全退出或后台驻留选项。
*   post-gate 无公网时 **MUST** 保证本地编辑能力；完整本地索引能力仍受 profile、search feature 与资源预算约束。
*   恢复网络后 **SHOULD** 增量同步；冲突策略以本地优先。
*   体积 **MUST** 控制在可接受范围，避免 UI 框架臃肿。

# 08_ui_design_03_mobile.md - 移动端设计 (Mobile UI)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Counterpart Feature`: `docs/features/08_ui_design.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/05_ui.md`, `docs/acceptance-cases/13_ui_mobile_chat_regression.md`
- `Primary Code Areas`: `apps/web/src/components/mobile_layout/`, `apps/web/src/components/`, `apps/mobile/`

本章定义 Mobile content-first 适配策略。规范性用语继承 `01_terminology.md`。

> **Current Native Boundary**：Mobile native 是壳层、生命周期与本机 service 绑定层，只表达 service readiness/offline，不拥有业务 authority。
> **Post-Gate Target**：Mobile 端目标采用 **Tauri v2 Mobile packaging** 外壳，共享 Web 前端；完整离线 packaging/readiness 必须等 native-packaging 与 process adapter gate 打开后验收。

> **Web 映射**：当 Web 端 $W_{view} \le 768px$ 时，界面 **MUST** 遵循本章 Mobile 规范。

## 1. 原生适配器边界 {#mobile-current-native-boundary}

*   Web 端小屏视口 **MUST** 映射到 Mobile 交互规范。
*   Mobile native adapter 第一阶段只允许承担：拉起受控内嵌服务、注入本机 service endpoint/session、报告 readiness/offline 状态、转发前后台、安全区域与软键盘等有限平台事件。
*   默认构建 **MUST** 保持 no-Tauri Mobile skeleton；真实 `tauri` / `tauri-build` dependency 只能在 `native-packaging` feature 与独立 gate 打开后引入。
*   mobile process adapter **MUST** 等 process adapter gate 显式打开后才能启动、持有或重启后端子进程。
*   recovery bootstrap 只能表达 `service_offline`、`foreground_reprobe` 与 `session_invalid` 等结构化状态；后台恢复失败 **MUST NOT** 被伪装成普通断网。
*   Mobile native adapter **MUST NOT** 自行定义 Ledger/Vault authority、schema migration、source-control 语义、同步合并语义或搜索索引语义；这些仍归 core/server。
*   UI readiness **MUST** 等待内嵌服务完成 loopback/IPC endpoint 与认证会话绑定后再打开主界面；后台/离线状态不得导致本地编辑进入未声明的半可写状态。

### 1.1 Minimal Native Adapter Contract {#mobile-native-adapter-contract}

Mobile native adapter 与 Desktop 共用 `08_ui_design_02_desktop.md#desktop-native-adapter-contract` 的 authority 边界：native 壳层只负责进程、平台能力与本机 service 绑定，不拥有 ledger/vault/source-control/search 的业务真相。

Packaging dependency gate 见 `14_tech_stack.md#native-packaging-dependency-gate`。

**Adapter inputs**:

*   `profile/config/vault/ledger` 选择必须在 service boot 前完成；Web 运行后 native 层不得直接改写后端路径或 repo scope。
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

*   native 层不得直接写 ledger/vault/source-control/search index。
*   native 层不得直接操作 `.notegit/` 或 `.git/` 来伪造 source-control 成功。
*   safe-area、keyboard、foreground/background、network online/offline 事件不得被解释成业务可写状态。

**Pre-Gate Acceptance Contract**:

*   cold start service bind 失败时显示恢复入口，不进入半可写 UI。
*   session invalid 时进入 `Unauthorized` 并停止普通重连。
*   network offline 但 service/session/writer ready 时，本地编辑继续可用。
*   background/resume 后必须重新握手，stale `scope_nonce` 写入被拒绝。

### 1.2 Embedded Service Supervisor Contract {#mobile-service-supervisor-contract}

Mobile 与 Desktop 共用 `08_ui_design_02_desktop.md#desktop-service-supervisor-contract`
的 supervisor 状态机，但 Mobile 额外保持生命周期约束：

*   `EndpointHealthy` 和 `SessionHandoffReady` 不得绕过 `ForegroundReprobe`；从后台恢复后仍必须重新 probe auth、node role、WS repo handshake 与 current `scope_nonce`。
*   `NetworkOffline` 仍只是公网提示；只有 embedded service health probe 失败才可进入 `ServiceOffline`。
*   `BackgroundSuspended` 不得清空 pending overlay，也不得把旧 supervisor session handoff 直接视为可写。
*   Health probe、retry budget、session handoff failure 分类与 Desktop 一致：bind/probe/process-exit 可预算内 retry，session handoff failure fatal。
*   supervisor 不得写 ledger/vault/source-control/search index/`.git`/`.notegit`。

### 1.3 Process Adapter Gate {#mobile-process-adapter-decision}

Mobile process adapter gate 默认关闭；在 gate 未经单独设计、评审与验收前，真实 mobile child-process runtime **MUST NOT** 进入默认 no-Tauri Mobile skeleton。

Gate policy 必须满足：

*   `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY.decision =
    DeferredUntilPackagingGate`
*   `child_process_runtime_enabled = false`
*   `packaging_gate_required = true`
*   `authority_writes_allowed = false`

真实 mobile process adapter 必须位于 Mobile native adapter 的 `native-packaging` feature 后，只做受控 spawn/probe/session/restart wiring；不得绕过 foreground reprobe、writer-ready 或 repo scope gate。

### 1.4 Mobile Packaging Scaffold {#mobile-packaging-scaffold}

Mobile packaging scaffold 只描述移动壳层的 post-gate 目标能力，**MUST NOT** 被解释为 packaging gate 已显式启用：

*   dependency batch: `tauri` runtime crate + `tauri-build` build crate。
*   packaging capability 只覆盖移动壳层能力：WebView shell、permission bridge、share sheet、
    deeplink、file picker、push notification、store package。
*   packaging scaffold 不得获得 ledger/vault/source-control/search index/`.git`/`.notegit`
    authority；这些业务真相仍只归 core/server。
*   lifecycle correctness 仍由 no-packaging mobile skeleton tests 保证：background/resume 后必须
    fresh reprobe auth、node role、WS repo handshake 与 current `scope_nonce`，packaging 不得绕过。
*   `scripts/check-native-track-boundary.sh` 必须继续阻止真实 packaging dependency 或 import
    在门禁打开前泄漏到 workspace root、core、cli、web 或 native 默认构建。

### 1.5 Mobile Packaging Dependency Gate {#mobile-packaging-dependency-gate-decision}

Mobile packaging dependency gate 默认关闭；在 gate 未经单独设计、评审与验收前，真实 Tauri Mobile runtime
dependency **MUST NOT** 进入默认 workspace 构建。

Gate policy 必须满足：

*   `CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY.decision =
    DeferredUntilRuntimeBatch`
*   `real_tauri_dependencies_allowed = false`
*   `default_build_remains_no_tauri = true`
*   `native_feature_gate_required = true`
*   `authority_writes_allowed = false`

Gate 打开时 **MUST** 先更新边界脚本，并继续保证 foreground reprobe、writer-ready 与
repo scope gate 不被 native runtime 绕过。

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

| `⇥` Tab | `H`ead | `•` List | `☑` Task | `B`old | `I`talic | `<>` Code | `↩` Undo |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| Indent | `#` | `-` | `[ ]` | `**` | `_` | \` | Cmd+Z |

**Technical Constraint**:
必须使用 `visualViewport` API 监听键盘高度变化，动态调整 Toolbar 的 `bottom` 偏移量，防止被键盘遮挡。
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
*   **Outline Drawer**:
    *   内容：标题结构、大纲条目。
    *   行为：点击条目后自动收起并滚动定位。

### 5.4 编辑器区 (Editor)
*   **Mode**: 单列布局，支持全屏编辑。
*   **Diff**: 强制 Unified View。
*   **Selection**: 单指选区，长按弹出操作菜单。

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

### 9.2 内嵌服务 (Embedded Service)
*   **Rule**: post-gate 后端服务 **MUST** 内嵌到安装包中，应用启动时自动拉起。
*   **Local API**: 前端通过本机回环接口访问内嵌服务，禁止依赖公网。

### 9.2.1 服务启动流程 (Service Boot)
*   **Rule**: post-gate App 启动 **MUST** 先拉起内嵌服务，再启动 UI。
*   **Port**: 端口 **MUST** 使用本机随机可用端口并保存在运行时内存中。
*   **Lifecycle**: App 进入后台时 **SHOULD** 降低服务资源占用；恢复前台时自动唤醒。
*   **Port Conflict**: 若端口占用，**MUST** 自动回退到新的可用端口并重新绑定。

### 9.2.2 本地通信策略 (Local IPC)
*   **Default**: 本机回环 HTTP/WS（`127.0.0.1`）优先。
*   **Fallback**: 若平台限制端口访问，**MUST** 提供进程内通道 (IPC) 替代方案。
*   **Security**: 本地通信 **MUST** 禁止跨进程未授权访问。
*   **Auth**: IPC **MUST** 具备进程级鉴权与会话绑定。

### 9.2.3 端口绑定安全 (Port Binding Security)
*   **Rule**: 服务端 **MUST** 仅监听 `127.0.0.1`。
*   **Firewall**: **SHOULD** 显式阻断非回环访问。

### 9.3 离线优先 (Offline-First)
*   **Rule**: post-gate 无公网时 **MUST** 提供本地读写能力；完整后台/索引/同步能力仍受平台 lifecycle、profile、feature 与资源预算约束。
*   **Sync**: 网络恢复后执行增量同步，失败时不影响本地编辑。

### 9.3.1 数据持久化 (Persistence)
*   **Rule**: 所有内容 **MUST** 落盘到本地数据库与 Vault。
*   **Crash Safety**: 崩溃后 **MUST** 可恢复到最后一次持久化状态。
*   **Migration Boundary**: 移动端 UI **MUST NOT** 自行定义存储迁移语义；涉及 Ledger / Vault Schema 的升级必须遵循 `04_storage.md` 的 `Copy & Rebuild` 策略，失败时进入显式恢复流程而不是静默自动回滚。

### 9.3.2 加密策略 (Encryption)
*   **At-Rest**: 本地存储 **MUST** 支持加密（密钥绑定设备安全模块）。
*   **In-Memory**: 解密后的明文 **SHOULD** 尽量短时保留。
*   **Key Rotation**: **MUST** 支持密钥轮换与失效，轮换过程不得破坏现有数据。
*   **Recovery**: **MUST** 提供密钥恢复策略，避免单点损坏。

### 9.3.3 备份与导出 (Backup & Export)
*   **Backup**: **MUST** 支持本地加密备份。
*   **Export**: **SHOULD** 支持单文档/全量导出。

### 9.3.4 权限与审计 (Permissions & Audit)
*   **Rule**: 本地操作 **MUST** 具备最小权限原则。
*   **Audit**: **SHOULD** 记录关键操作日志（创建/删除/导出/恢复）。

### 9.3.5 恢复演练 (Recovery Drill)
*   **Rule**: 版本升级 **SHOULD** 提供可执行的恢复演练流程。
*   **Goal**: 发生故障时可快速回退到稳定版本。

### 9.4 体积与性能约束 (Size & Performance)
*   **Size**: 体积 **MUST** 可控，避免引入重型依赖。
*   **Perf**: 输入延迟与滚动流畅性必须优先保障。

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

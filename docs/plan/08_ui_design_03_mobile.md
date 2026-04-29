# 08_ui_design_03_mobile.md - 移动端设计 (Mobile UI)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Counterpart Feature`: `docs/features/08_ui_design.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/05_ui.md`, `docs/acceptance-cases/13_ui_mobile_chat_regression.md`
- `Primary Code Areas`: `apps/web/src/components/mobile_layout/`, `apps/web/src/components/`, `apps/mobile/`

本节定义了 Mobile 端基于 **Content-First** 哲学的适配策略。

> **Tauri-Based**: Mobile 端采用 **Tauri v2 Mobile** 外壳，前端代码与 Web 端共享。
> **Offline-First**: Mobile 端 **MUST** 在无网络环境下保持完整可用。

> **Web Mapping**: 当 Web 端 $W_{view} \le 768px$ 时，界面 **MUST** 遵循本章 Mobile 规范。

## 0. Current Native Boundary (2026-04-29) {#mobile-current-native-boundary}

当前代码状态：

*   Web 端 Mobile responsive shell 已存在，并作为 Mobile 交互规范的当前可验收映射。
*   `apps/mobile` 已提供最小 native shell skeleton：受控 loopback endpoint、session 绑定、Web bootstrap 注入、background/suspended/resumed/foreground reprobe、service offline 与 session invalid recovery 状态机。它不是完整 Tauri Mobile 应用。
*   Tauri v2 Mobile packaging、系统权限桥接、推送、原生文件选择器与应用商店分发仍是 future work；当前仓库不得把这些视为已实现能力。
*   packaging runtime 只能在 `apps/mobile` 的 `native-packaging` feature 后引入；默认构建必须保持 no-Tauri Mobile skeleton，以便快速验证 lifecycle/session/readiness contract。
*   Mobile native adapter 的第一阶段职责只允许是：拉起受控内嵌服务、注入本机服务 endpoint/session、报告 service readiness/offline 状态、转发前后台与安全区域等有限平台事件。
*   `deve_core::native_adapter::NativeServiceSupervisor` 已提供 no-Tauri supervisor contract：service start、health probe、session handoff、retry budget 与 offline classification；mobile shell 在此基础上继续保留 background/resume fresh reprobe 规则。
*   Web 已支持 mobile/native recovery bootstrap：`service_offline`、`foreground_reprobe` 与 `session_invalid` 会映射到明确 UI/写入门禁状态，而不是普通断网。
*   Mobile native adapter **MUST NOT** 自行定义 Ledger/Vault authority、schema migration、source-control 语义、同步合并语义或搜索索引语义；这些仍归 core/server。
*   UI readiness **MUST** 等待内嵌服务完成 loopback/IPC endpoint 与认证会话绑定后再打开主界面；后台/离线状态不得导致本地编辑进入未声明的半可写状态。

### 0.1 Minimal Native Adapter Contract {#mobile-native-adapter-contract}

Mobile native adapter 与 Desktop 共用 `08_ui_design_02_desktop.md#desktop-native-adapter-contract`
的 authority 边界：native 壳层只负责进程、平台能力与本机 service 绑定，不拥有
ledger/vault/source-control/search 的业务真相。

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

`RuntimeReady` 的最小条件与 Desktop 一致：本机 endpoint 可达、`/api/auth/status`
有效、`/api/node/role` 可读取、当前 repo 已完成 ws handshake，且写入路径满足
`writer_ready(repo_id, scope_nonce)`。`SessionInvalid` 必须进入 `Unauthorized`；
移动端后台恢复失败不得被伪装成普通断网。

**Endpoint/session injection rules**:

*   Native 壳必须在 Web connection manager 启动前注入 `http_base/ws_base` 与 session 绑定状态；优先使用内存 bridge 或初始 HTML bootstrap。
*   Native 壳也可以注入只含 `service_state` 的 recovery bootstrap；该 payload 只能表达 `service_offline`、`foreground_reprobe` 或 `session_invalid`，不得携带 token、session secret、服务失败 reason 或 repo 写权限。
*   `?ws_port=` 只能作为开发期 fallback。mobile production 不得让 Web 端枚举、猜测或扫描本机端口。
*   session 绑定完成前不得打开可写主界面；后台恢复后必须重新 probe session、node role 与 ws repo handshake。

**Offline/background semantics**:

*   `NetworkOffline` 只表示公网不可用；如果 embedded service、session 与 writer gate 仍 ready，本地编辑仍可继续。
*   `BackgroundSuspended` 只允许降低资源占用或暂停远端同步，不得丢弃未 ack 的本地 pending overlay。
*   `ForegroundReprobe` 必须重新执行 `/api/auth/status`、`/api/node/role` 与 repo handshake；旧 `scope_nonce` 不得自动恢复写态。
*   `ServiceOffline` 表示本机后端不可达；UI 必须进入恢复/只读状态，不得声称 offline-first 仍可写。

**Forbidden native shortcuts**:

*   native 层不得直接写 ledger/vault/source-control/search index。
*   native 层不得直接操作 `.notegit/` 或 `.git/` 来伪造 source-control 成功。
*   safe-area、keyboard、foreground/background、network online/offline 事件不得被解释成业务可写状态。

**Acceptance before native implementation**:

*   cold start service bind 失败时显示恢复入口，不进入半可写 UI。
*   session invalid 时进入 `Unauthorized` 并停止普通重连。
*   network offline 但 service/session/writer ready 时，本地编辑继续可用。
*   background/resume 后必须重新握手，stale `scope_nonce` 写入被拒绝。

### 0.1.1 Embedded Service Supervisor Contract {#mobile-service-supervisor-contract}

Mobile 与 Desktop 共用 `08_ui_design_02_desktop.md#desktop-service-supervisor-contract`
的 supervisor 状态机，但 Mobile 额外保持生命周期约束：

*   `EndpointHealthy` 和 `SessionHandoffReady` 不得绕过 `ForegroundReprobe`；从后台恢复后仍必须重新 probe auth、node role、WS repo handshake 与 current `scope_nonce`。
*   `NetworkOffline` 仍只是公网提示；只有 embedded service health probe 失败才可进入 `ServiceOffline`。
*   `BackgroundSuspended` 不得清空 pending overlay，也不得把旧 supervisor session handoff 直接视为可写。
*   Health probe、retry budget、session handoff failure 分类与 Desktop 一致：bind/probe/process-exit 可预算内 retry，session handoff failure fatal。
*   supervisor 不得写 ledger/vault/source-control/search index/`.git`/`.notegit`。

### 0.2 Mobile Packaging Scaffold {#mobile-packaging-scaffold}

`apps/mobile` 当前提供 `native-packaging` feature 后的 packaging scaffold，但仍不引入
真实 Tauri Mobile runtime dependency。该 scaffold 只用于固定下一批 packaging dependency
decision 的验收输入：

*   planned dependency batch: `tauri` runtime crate + `tauri-build` build crate，状态仍为 `planned`。
*   packaging capability 只覆盖移动壳层能力：WebView shell、permission bridge、share sheet、
    deeplink、file picker、push notification、store package。
*   packaging scaffold 不得获得 ledger/vault/source-control/search index/`.git`/`.notegit`
    authority；这些业务真相仍只归 core/server。
*   lifecycle correctness 仍由 no-packaging mobile skeleton tests 保证：background/resume 后必须
    fresh reprobe auth、node role、WS repo handshake 与 current `scope_nonce`，packaging 不得绕过。
*   `scripts/check-native-track-boundary.sh` 必须继续阻止真实 packaging dependency 或 import
    在门禁打开前泄漏到 workspace root、core、cli、web 或 native 默认构建。

## 1. Normative Language (规范性用语)
*   **MUST**: 绝对要求。
*   **SHOULD**: 强烈建议。

## 2. Responsive Architecture {#mobile-responsive-layout}

### 1.1 布局状态机 (Layout State Machine)
系统布局 $L$ 根据视口宽度 $W_{view}$ 在两种状态间切换：

*   **Desktop State**: $W_{view} > 768px \implies$ Grid Layout.
*   **Mobile State**: $W_{view} \le 768px \implies$ Stack Layout.

### 1.2 视口配置 (Viewport Configuration)
为了适配刘海屏 (Notch) 并防止 iOS 自动缩放，HTML Header **MUST** 包含：

```html
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
```

并且所有固定定位元素 **MUST** 使用 CSS `env()` 适配安全区域：
*   `padding-top: env(safe-area-inset-top)`
*   `padding-bottom: env(safe-area-inset-bottom)`

## 3. Interaction Design {#mobile-interaction-design}

### 2.1 导航策略 (Navigation)
移动端移除常驻侧边栏，改为 **Drawer (抽屉)** 模式。

*   **Left Drawer (Sidebar)**:
*   **Trigger**: 左上角汉堡菜单 (`≡`) 或 **屏幕左边缘右滑 (Edge Swipe)**。
*   **Visual**: 覆盖在编辑器之上，背景带有半透明 Backdrop (`z-index: 100`).
*   **Right Drawer (Outline)**:
*   **Trigger**: 编辑器内容区右上角的 `Toggle Outline` 浮动图标（非 Top Bar）或 **屏幕右边缘左滑**。

### 2.2 面板宽度策略 (Panel Width Policy)

*   **Resizable Handles**: 移动端 **SHOULD NOT** 显示左右拉伸手柄。
*   **Persistence**: 仍可读取已保存的桌面宽度，但移动端不提供调整入口。
*   **Outer Gutter**: 移动端 **SHOULD NOT** 提供外边距拖拽。

### 2.3 虚拟辅助键盘栏 (Mobile Toolbar)
为了解决移动端输入 Markdown 符号的痛点，系统 **MUST** 在软键盘上方渲染 Accessory View。

**Key Layout (Visual Representation)**:

| `⇥` Tab | `H`ead | `•` List | `☑` Task | `B`old | `I`talic | `<>` Code | `↩` Undo |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| Indent | `#` | `-` | `[ ]` | `**` | `_` | \` | Cmd+Z |

**Technical Constraint**:
必须使用 `visualViewport` API 监听键盘高度变化，动态调整 Toolbar 的 `bottom` 偏移量，防止被键盘遮挡。
Toolbar **SHOULD** 仅在软键盘可见时显示；软键盘弹出时底部状态栏可暂时让位以优先输入。

### 2.4 手势系统 (Gesture System)
仅支持轻量级 Edge Swipe，参数定义如下：
*   $Zone_{edge} = 20px$ (从屏幕边缘起算的响应区)。
*   $Threshold_{swipe} = 50px$ (触发滑动的最小距离)。
*   **Interactive Safety**: Edge Swipe **MUST NOT** 抢占靠边可交互控件的真实点击，例如 `File tree`、`Toggle Outline` 等按钮。

## 4. Visual Adaptations

### 3.1 布局约束
*   **Diff View**: 移动端 **MUST NOT** 使用左右并排 (Side-by-Side) 对比，而应强制回退到 **Unified View** (单列混合)。
*   **Font Size**: 默认字号 **SHOULD** 设为 `16px` 以避免 iOS Safari 输入时强制放大页面。

### 3.2 只读模式指示器 (Spectator Indicator)
在 Spectator Mode 下，顶部导航栏下方 **MUST** 插入一条醒目的橙色横幅 (`Height: 24px`)，提示 "Read-Only Mode"。

## 5. Mobile UI Layout

### 4.1 结构层级 (Hierarchy)
*   **Top App Bar**: 固定顶部，包含导航与核心操作。
*   **Content Stack**: 单列内容区，默认全屏编辑器。
*   **Bottom Bar**: 与 Desktop 功能对齐（Branch/连接状态/加载状态/历史条/统计），移动端允许多行折叠布局。
    *   默认折叠态 **MUST** 仅显示一行：`Branch / Ready / Words / Lines / Col`。
    *   通过右侧箭头按钮展开详情；再次点击或点击状态栏外区域自动收起。
    *   折叠态信息 **SHOULD** 无需横向滚动。
    *   分支名在折叠态 **SHOULD** 自动截断，避免挤压状态与统计信息。

### 4.2 顶部导航栏 (Top App Bar)
*   **Left**: Hamburger Menu (`≡`) 打开 Sidebar Drawer。
    *   该入口按钮文案/可访问性语义建议统一为 `File tree`。
*   **Center**: 文档标题/仓库名（省略溢出）。
*   **Right**: Home / Open / Command（与 Desktop 顶栏语义一致）。

### 4.3 Drawer 规范 (Side Drawers)
*   **Sidebar Drawer**:
    *   内容：文件树、快速操作、新建。
    *   关闭按钮建议使用 `X` 图标而非文本 `Close`，以符合移动端通用习惯。
    *   行为：点击文件后自动收起。
    *   `More(...)` 菜单 **MUST** 复用桌面端语义：整行点击切换视图，`Pin/Unpin` 仅修改固定状态，不得伪装成“点击无反应”。
*   **Outline Drawer**:
    *   内容：标题结构、大纲条目。
    *   行为：点击条目后自动收起并滚动定位。

### 4.4 编辑器区 (Editor)
*   **Mode**: 单列布局，支持全屏编辑。
*   **Diff**: 强制 Unified View。
*   **Selection**: 单指选区，长按弹出操作菜单。

### 4.5 快捷入口 (Quick Actions)
*   **Search**: 打开 Quick Open / Command Palette（移动端应为底部抽屉）。
*   **Sync**: 可在 More 菜单中触发。

## 6. Key Flows

### 5.1 打开文档
1. 点击 Hamburger -> 打开 Sidebar Drawer。
2. 选择文档 -> Drawer 自动收起 -> Editor 渲染。

### 5.2 查看大纲
1. 点击编辑器内容区右上角 `Toggle Outline` 图标 -> 打开右侧 Drawer。
2. 点击条目 -> Drawer 自动收起 -> Editor 滚动定位。

### 5.3 搜索/命令
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

## 9. Implementation Strategy

### 4.1 移动端 UI 方案
*   **Rule**: Mobile 采用 **Tauri v2 Mobile** 作为外壳（WKWebView / Android WebView），前端代码与 Web 端共享，配合原生层访问摄像头/文件系统/推送等系统 API。
*   **Consistency**: 交互与布局规则 **MUST** 与本章一致，行为不以 Web 端为准。

### 7.2 内嵌服务 (Embedded Service)
*   **Rule**: 后端服务 **MUST** 内嵌到安装包中，应用启动时自动拉起。
*   **Local API**: 前端通过本机回环接口访问内嵌服务，禁止依赖公网。

### 7.2.1 服务启动流程 (Service Boot)
*   **Rule**: App 启动 **MUST** 先拉起内嵌服务，再启动 UI。
*   **Port**: 端口 **MUST** 使用本机随机可用端口并保存在运行时内存中。
*   **Lifecycle**: App 进入后台时 **SHOULD** 降低服务资源占用；恢复前台时自动唤醒。
*   **Port Conflict**: 若端口占用，**MUST** 自动回退到新的可用端口并重新绑定。

### 7.2.2 本地通信策略 (Local IPC)
*   **Default**: 本机回环 HTTP/WS（`127.0.0.1`）优先。
*   **Fallback**: 若平台限制端口访问，**MUST** 提供进程内通道 (IPC) 替代方案。
*   **Security**: 本地通信 **MUST** 禁止跨进程未授权访问。
*   **Auth**: IPC **MUST** 具备进程级鉴权与会话绑定。

### 7.2.3 端口绑定安全 (Port Binding Security)
*   **Rule**: 服务端 **MUST** 仅监听 `127.0.0.1`。
*   **Firewall**: **SHOULD** 显式阻断非回环访问。

### 7.3 离线优先 (Offline-First)
*   **Rule**: 无网络时 **MUST** 提供完整读写能力。
*   **Sync**: 网络恢复后执行增量同步，失败时不影响本地编辑。

### 7.3.1 数据持久化 (Persistence)
*   **Rule**: 所有内容 **MUST** 落盘到本地数据库与 Vault。
*   **Crash Safety**: 崩溃后 **MUST** 可恢复到最后一次持久化状态。
*   **Migration Boundary**: 移动端 UI **MUST NOT** 自行定义存储迁移语义；涉及 Ledger / Vault Schema 的升级必须遵循 `04_storage.md` 的 `Copy & Rebuild` 策略，失败时进入显式恢复流程而不是静默自动回滚。

### 7.3.2 加密策略 (Encryption)
*   **At-Rest**: 本地存储 **MUST** 支持加密（密钥绑定设备安全模块）。
*   **In-Memory**: 解密后的明文 **SHOULD** 尽量短时保留。
*   **Key Rotation**: **MUST** 支持密钥轮换与失效，轮换过程不得破坏现有数据。
*   **Recovery**: **MUST** 提供密钥恢复策略，避免单点损坏。

### 7.3.3 备份与导出 (Backup & Export)
*   **Backup**: **MUST** 支持本地加密备份。
*   **Export**: **SHOULD** 支持单文档/全量导出。

### 7.3.4 权限与审计 (Permissions & Audit)
*   **Rule**: 本地操作 **MUST** 具备最小权限原则。
*   **Audit**: **SHOULD** 记录关键操作日志（创建/删除/导出/恢复）。

### 7.3.5 恢复演练 (Recovery Drill)
*   **Rule**: 版本升级 **SHOULD** 提供可执行的恢复演练流程。
*   **Goal**: 发生故障时可快速回退到稳定版本。

### 7.4 体积与性能约束 (Size & Performance)
*   **Size**: 体积 **MUST** 可控，避免引入重型依赖。
*   **Perf**: 输入延迟与滚动流畅性必须优先保障。

## 10. Web Mobile Alignment Checklist

> 目标：在 `W_view <= 768px` 的 Web 视口中，持续对齐本章 Mobile 规范。

### 8.1 当前已完成
*   移动布局模块化拆分（`mobile_layout/{mod,header,content,footer,effects,gesture}`）。
*   Drawer 模块化拆分（`mobile_layout/drawers/{mod,left,right}`）。
*   左右抽屉、边缘滑动开关、抽屉互斥、抽屉打开时 body 锁滚。
*   Safe-area 适配、Bottom Sheet 搜索面板、空态与 CTA、基础触控反馈。
*   边缘滑动与靠边交互控件冲突隔离，避免误吞 `File tree / Toggle Outline` 点击。
*   `More(...)` 菜单项点击与 `Pin/Unpin` 语义分离，并在点击后正确收起。

### 8.2 本轮优先对齐项
*   **Bottom Sheet 手势关闭**：
    *   **MUST** 具备下拉关闭阈值（避免轻微位移误关闭）。
    *   **MUST** 增加误触防抖（短时微位移不触发关闭）。
    *   **MUST** 处理与滚动冲突：仅在列表位于顶部且判定为下拉意图时才允许关闭。
*   **Drawer 可达性一致性**：
    *   **MUST** 统一标题栏与关闭按钮交互语义。
    *   **SHOULD** 保障触控命中高度不低于 `44px`。
*   **列表触控反馈一致性**：
    *   Sidebar / Outline / Search Result 的 `selected`、`hover`、`active` 语义 **MUST** 保持一致。

### 8.3 执行与验证
*   小步迭代，每轮改动后执行：`cargo clippy --all-targets --all-features -- -D warnings`。
*   保持低复杂度与模块化；超过 250 行需复查职责内聚，超过 500 行需拆分或说明例外。

### 8.4 本轮落地记录 (Web, 2026-02)
*   Top Sheet 关闭策略完成：阈值 `72px`、防抖 `<=90ms & <=20px`、以顶部把手/头部区域上滑关闭为主，避免与结果列表滚动冲突。
*   Drawer 与触控反馈完成一致化：标题栏/关闭按钮命中区 `44px+`，Sidebar/Outline/Search Result 的 `hover/active/selected` 语义对齐。
*   视口与阅读态完成：`meta viewport` 补齐、移动端 `Read-Only Mode` 24px 横幅、Diff 强制 Unified。
*   顶部与底部导航完成对齐：Top Bar 右侧改为 Home/Open/Command；Bottom Bar 对齐 Desktop 的 branch、状态、历史、统计（移动端多行布局）。
*   Outline 入口完成统一：取消 Top Bar 入口，改为内容区 `Toggle Outline` 浮动图标（开时位于右抽屉左上角，关时位于内容区右上角）。
*   输入态冲突完成收敛：Accessory Toolbar 仅在软键盘出现时显示，键盘出现时隐藏底部状态栏；`<=360px` 极窄屏启用专用排版（历史控制拆分为两行，统计标签压缩）。
*   视觉一致性微调：Top Sheet 增加轻量遮罩与模糊背景，Outline 浮动开关位置与动画节律对齐，Bottom Bar 信息胶囊统一浅浮雕层次。
*   Bottom Bar 交互升级：新增折叠/展开箭头，折叠态固定单行展示 `Branch/Ready/Words/Lines/Col`，展开态显示加载信息与历史控制；点击栏外区域自动收起。
*   动效节律对齐：Drawer、Top Sheet、Outline Toggle、按钮反馈统一为 `duration-200 + ease-out`。
*   I18n 对齐：移动端新增文案 `File tree`、`Close file tree`、`Toggle status details`、`Files`、`Outline`、`Outline unavailable`、`No headings found` 已接入 i18n key，移除对应硬编码。
*   复杂度治理：`search_box/logic.rs` 已拆分为 `logic/{providers,selection,actions,execute}.rs`，降低单文件复杂度并便于后续移动端交互迭代。
*   交互丝滑度增强：Top Sheet 新增手势跟手位移（上滑拖拽过程实时反馈），未达到关闭阈值时平滑回弹。
*   AI Chat 移动端接入：新增可折叠入口（`AI` 胶囊按钮），点击后进入同页全屏 Chat 页面（非浏览器新窗口），右上角关闭后返回原编辑页面。
*   AI Chat 输入与层级收敛：Chat Sheet 展开时自动让位 Bottom Bar 与移动辅助键盘栏，键盘弹起时以 `visualViewport` 偏移贴合，保证发送按钮可达。
*   AI Chat 消息可读性增强：移动端气泡宽度与边距重排、长文本强制换行、代码块横向滚动、消息时间戳展示；流式状态、加载态、错误态、重试态在移动端统一可见。
*   AI Chat i18n 收口：聊天面板标题、角色名、输入占位、发送/失败/重试、空态提示、`Apply` 代码按钮与移动端切换文案均接入 `t::chat::*`，组件不再新增硬编码文案。
*   AI Chat 全屏交互收敛：移除半屏抓手关闭手势，统一采用顶部关闭按钮返回，减少误触并对齐移动端“页面级”交互预期。
*   Mobile Drawer 与桌面对齐：左抽屉新增图标化视图栏（Explorer/Search/Source Control/Extensions）+ `More(...)` 入口，支持固定项（Pin/Unpin）管理并复用桌面端 `pinned_views` 状态。
*   More 菜单交互收敛：点击菜单项后立即关闭；点击外部与 `Esc` 均可关闭；抽屉关闭时菜单强制收口，避免状态残留。
*   Diff 场景冲突治理：移动端打开 Diff 时隐藏 AI Chat 入口与移动辅助键盘栏，减少层级遮挡和误触。
*   Diff 可回归钩子补齐：移动端 Diff 根节点与关闭按钮补充稳定选择器（`.diff-view-mobile` / `.diff-close-button`），用于验收脚本可靠定位。
*   细节识别优化：Bottom Bar 紧凑分支名采用前后保留压缩（如 `feature...23ab`），提升分支辨识度。
*   手势物理感增强：Top Sheet 新增快甩关闭判定（速度阈值）与阻尼位移（越拉越难）。

## 11. SHOULD Mapping Matrix (Web Mobile)

| 条目 | 规范原文 (SHOULD) | 代码路径 | 状态 | 备注 |
| :--- | :--- | :--- | :--- | :--- |
| MOB-SHOULD-001 | Resizable Handles: 移动端 SHOULD NOT 显示左右拉伸手柄 | `apps/web/src/components/main_layout.rs` | 已实现 | `is_mobile` 分支渲染 `MobileLayout`，不挂载桌面拖拽手柄 UI。 |
| MOB-SHOULD-002 | Outer Gutter: 移动端 SHOULD NOT 提供外边距拖拽 | `apps/web/src/components/main_layout.rs` | 已实现 | 外边距拖拽仅在 `DesktopLayout` 生效。 |
| MOB-SHOULD-003 | Font Size: 默认字号 SHOULD 设为 16px | `apps/web/src/editor/mod.rs` | 部分实现 | 编辑器基础字号尚未统一锁定为 16px（后续可在编辑器容器样式或主题变量中强制）。 |
| MOB-SHOULD-004 | App 后台时服务 SHOULD 降低资源占用 | N/A (Web Scope) | 不适用 | 属于原生 Mobile App 进程生命周期，不在 Web 映射实现范围。 |
| MOB-SHOULD-005 | Firewall SHOULD 显式阻断非回环访问 | N/A (Embedded Service) | 不适用 | 属于移动端内嵌服务与系统防火墙策略。 |
| MOB-SHOULD-006 | Export SHOULD 支持单文档/全量导出 | N/A (Mobile Native Service) | 不适用 | 属于原生端导出与存储能力。 |
| MOB-SHOULD-007 | Audit SHOULD 记录关键操作日志 | N/A (Core/Service) | 不适用 | 属于后端审计链路，不在 Web UI 直接实现。 |
| MOB-SHOULD-008 | Recovery Drill SHOULD 提供恢复演练流程 | N/A (Release/Ops) | 不适用 | 属于发布与运维流程规范。 |

### 9.1 与本轮实现直接相关的 SHOULD 细化

| 条目 | 代码路径 | 状态 | 备注 |
| :--- | :--- | :--- | :--- |
| MOB-UX-SHOULD-001 | `apps/web/src/components/mobile_layout/drawers/left.rs`, `apps/web/src/components/mobile_layout/drawers/right.rs`, `apps/web/src/components/mobile_layout/header.rs` | 已实现 | 触控命中区统一到 `44px+`，标题栏/关闭按钮语义一致。 |
| MOB-UX-SHOULD-002 | `apps/web/src/components/search_box/ui.rs`, `apps/web/src/components/search_box/sheet_gesture.rs` | 已实现 | Bottom Sheet 手势关闭已做阈值/防抖/滚动冲突判定。 |
| MOB-UX-SHOULD-003 | `apps/web/src/components/sidebar/item.rs`, `apps/web/src/components/outline.rs`, `apps/web/src/components/search_box/result_item.rs` | 已实现 | 列表项 `hover/active/selected` 语义对齐，移动端优先 `active`。 |

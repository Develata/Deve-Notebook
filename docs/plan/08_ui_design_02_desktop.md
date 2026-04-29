# 08_ui_design_02_desktop.md - 桌面端设计 (Desktop UI)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Counterpart Feature`: `docs/features/08_ui_design.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/05_ui.md`
- `Primary Code Areas`: `apps/web/src/components/`, `apps/web/src/hooks/use_core/`, `apps/desktop/`

本节定义了 Desktop 端的”驾驶舱”布局规范与交互逻辑。

> **Tauri-Based**: Desktop 端采用 **Tauri v2** 外壳，前端代码与 Web 端共享。
> **Offline-First**: Desktop 端 **MUST** 在无网络环境下保持完整可用。

> **Web Mapping**: 当 Web 端 $W_{view} > 768px$ 时，界面 **MUST** 遵循本章 Desktop 规范。

## 0. Current Native Boundary (2026-04-29) {#desktop-current-native-boundary}

当前代码状态：

*   Web 端 Desktop responsive shell 已存在，并作为 Desktop 交互规范的当前可验收映射。
*   `apps/desktop` 已提供最小 native shell skeleton：受控 loopback endpoint、session 绑定、Web bootstrap 注入与 offline/session-invalid recovery 状态机。它不是完整 Tauri 应用。
*   Tauri v2 native packaging、原生菜单栏、系统托盘、安装包与自动更新仍是 future work；当前仓库不得把这些视为已实现能力。
*   packaging runtime 只能在 `apps/desktop` 的 `native-packaging` feature 后引入；默认构建必须保持 no-Tauri skeleton，以便快速验证 adapter/session/readiness contract。
*   Native adapter 的第一阶段职责只允许是：拉起受控内嵌服务、注入本机服务 endpoint/session、报告 service readiness/offline 状态、转发有限平台事件。
*   `deve_core::native_adapter::NativeServiceSupervisor` 已提供 no-Tauri supervisor contract：service start、health probe、session handoff、retry budget 与 offline classification；desktop shell 只消费该 contract，不拥有业务 authority。
*   Web 已支持 native recovery bootstrap：`service_offline` 显示原生服务离线恢复状态，`session_invalid` 进入 `Unauthorized`，无效 endpoint/session-pending 不得回退端口扫描。
*   Native adapter **MUST NOT** 重新定义 Ledger/Vault authority、schema migration、source-control 语义或搜索索引语义；这些仍归 core/server。
*   UI readiness **MUST** 等待内嵌服务完成 loopback/IPC endpoint 与认证会话绑定后再打开主界面；失败时显示恢复入口而不是进入半可写状态。

### 0.1 Minimal Native Adapter Contract {#desktop-native-adapter-contract}

Desktop native adapter 是进程与平台壳层，不是业务 authority。第一阶段只允许把现有
Web shell 绑定到本机受控 service，并把 runtime 状态结构化交给 Web/application
control。

Packaging dependency gate 见 `14_tech_stack.md#native-packaging-dependency-gate`。

### 0.2 Desktop Packaging Scaffold {#desktop-packaging-scaffold}

当前 `apps/desktop` 已在 `native-packaging` feature 后提供 packaging scaffold，
但仍不引入 Tauri runtime dependency。该 scaffold 只描述首个桌面壳层批次：

*   Planned dependency batch: `tauri` + `tauri-build`，只能落在 `apps/desktop`。
*   Planned capabilities: window shell、menu bar、system tray、installer、auto-update。
*   Forbidden authorities: ledger、vault、source-control、search index、`.git` mirror、
    `.notegit`。
*   no-packaging skeleton tests 仍是 endpoint/session/bootstrap/readiness correctness 的
    authority；packaging acceptance 不得替代这些测试。

在真正添加 Tauri dependency 前，`scripts/check-native-track-boundary.sh` 必须继续阻止
Cargo dependency/import 泄漏。

### 0.3 Embedded Service Supervisor Contract {#desktop-service-supervisor-contract}

当前 desktop native skeleton 已接入共享 `NativeServiceSupervisor`，但仍不启动真实子进程。
该 contract 只固定 supervisor 状态机与 failure 分类，避免后续 Tauri/process integration
把 service readiness 与业务写权限混在一起：

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
*   supervisor 不得写 ledger/vault/source-control/search index/`.git`/`.notegit`。

**Adapter inputs**:

*   `profile/config/vault/ledger` 选择必须在 service boot 前完成，并传入后端启动参数；native 层不得在 Web 运行后直接改写这些路径。
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

`RuntimeReady` 的最小条件是：本机 endpoint 可达、`/api/auth/status` 有效、`/api/node/role`
可读取、当前 repo 已完成 ws handshake，且写入路径满足
`writer_ready(repo_id, scope_nonce)`。只要 `SessionInvalid` 出现，UI 必须进入
`Unauthorized`，不得包装成普通 `Disconnected`。

**Endpoint/session injection rules**:

*   Native 壳必须在 Web connection manager 启动前注入 `http_base/ws_base` 与 session 绑定状态；优先使用内存 bridge 或初始 HTML bootstrap。
*   Native 壳也可以注入只含 `service_state` 的 recovery bootstrap；该 payload 只能表达 `service_offline` 或 `session_invalid`，不得携带 token、session secret、服务失败 reason 或 repo 写权限。
*   `?ws_port=` 只能作为开发期 fallback。native production 不得让 Web 端枚举、猜测或扫描本机端口。
*   session 绑定完成前 Web shell 不得显示可写主界面；过期 session 必须走 `09_auth.md#unauthorized-disconnected-ui`。

**Offline/readiness semantics**:

*   `NetworkOffline` 只表示公网不可用；如果 embedded service、session 与 writer gate 仍 ready，Desktop 本地编辑仍可继续。
*   `ServiceOffline` 表示本机后端不可达；UI 必须进入恢复/只读状态，不得假装仍有本地 authority。
*   App 从后台/驻留状态恢复时必须重新 probe `/api/auth/status`、`/api/node/role`，并重新确认 ws repo handshake；旧 `scope_nonce` 不得自动恢复写态。

**Forbidden native shortcuts**:

*   native 层不得直接写 ledger/vault/source-control/search index。
*   native 层不得直接操作 `.notegit/` 或 `.git/` 来伪造 source-control 成功。
*   native 层不得把平台 online/offline、窗口焦点或 Tauri lifecycle 事件解释成业务可写状态。

**Acceptance before native implementation**:

*   service bind 失败时显示恢复入口，不进入半可写 UI。
*   session invalid 时进入 `Unauthorized` 并停止普通重连。
*   network offline 但 service/session/writer ready 时，本地编辑继续可用。
*   resume/restart 后必须重新握手，stale `scope_nonce` 写入被拒绝。

## 1. Normative Language (规范性用语)
*   **MUST**: 绝对要求。
*   **SHOULD**: 强烈建议。

## 2. The Cockpit Concept (布局哲学)

桌面端设计遵循 **"Information Stratification" (信息分层)** 原则，将界面划分为不同关注度的区域。

*   **L1 (Focus)**: 编辑区 (Editor)。绝对中心，无干扰。
*   **L2 (Context)**: 侧边栏 (Sidebar)。提供导航上下文 (Explorer, Outline)。
*   **L3 (Meta)**: 状态栏 (Status Bar)。提供系统元数据 (Git Branch, Sync Status)。
*   **L4 (Floating)**: 悬浮层 (Overlays)。按需出现的命令入口。

## 3. Dynamic Grid System

### 2.1 布局定义 (Layout Definition)
系统采用 5 列动态网格布局。形式化定义如下：

$$ Grid = [Col_{sidebar}, Col_{diff\_old}, Col_{editor}, Col_{outline}, Col_{chat}] $$

*   **Constraint**: $Col_{editor}$ (Col 3) 总是占据剩余空间 (`1fr`)。
*   **CSS Implementation**:
    ```css
    display: grid;
    grid-template-columns: var(--w-sidebar) var(--w-diff) 1fr var(--w-outline) var(--w-chat);
    ```

### 2.2 布局可视化 (Visualization)

**Main Workbench Structure**:

| Layer      | Col 1 (Resizable) | Col 2 (Fixed) | Col 3 (Flex) | Col 4 (Fixed) | Col 5 (Resizable) |
| :--------- | :---------------- | :------------ | :----------- | :------------ | :---------------- |
| **Top**    | `[Explorer]`      | `Old.rs`      | `New.rs`     | `Outline`     | `AI Chat`         |
| **Body**   | File Tree         | Read-Only     | Writable     | H1..H6        | Chat Log          |
| **Resize** | `[||]` Handle     | -             | -            | -             | `[||]` Handle     |

### 2.3 组件规范 (Component Specs)

*   **Primary Sidebar (Col 1)**:
    *   **Behavior**: **MUST** 支持拖拽调整宽度 (`180px` ~ `500px`)。
    *   **Persistence**: **MUST** 记住用户设置。
*   **Right Panel (Col 5)**:
    *   **Behavior**: **MUST** 支持拖拽调整宽度 (`240px` ~ `520px`)。
    *   **Persistence**: **MUST** 记住用户设置。
*   **Outer Gutter**:
    *   **Behavior**: **MUST** 支持拖拽调整主区域左右边距。
    *   **Persistence**: **MUST** 记住用户设置。
    *   **State**: 包含 `ActivityBar` (Icon Strip) 与 `SideView` (Content)。
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

### 3.1 视图结构 (View Structure)
定义源代码管理视图 $V_{sc}$ 为三个集合的并集：
$$ V_{sc} = S_{staged} \cup S_{unstaged} \cup H_{commits} $$

*   **Staged ($S_{staged}$)**: 已暂存的文件集合。支持 `Unstage All`。
*   **Unstaged ($S_{unstaged}$)**: 工作区的脏文件集合。支持 `Stage All` / `Discard All`。

### 3.2 变更状态可视化
每个变更项 $Item \in V_{sc}$ **MUST** 使用语义化颜色标记状态：

*   **Modified ($M$)**: Orange (`var(--color-modified)`).
*   **Added ($A$)**: Green (`var(--color-added)`).
*   **Deleted ($D$)**: Red (`var(--color-deleted)`).

## 5. Related Commands

*   `view.layout.toggle_sidebar`: 切换侧边栏可见性。
*   `view.layout.toggle_diff`: 切换 Diff/Editor 模式。
*   `git.stage_all`: 暂存所有更改。

## 6. Implementation Strategy

### 4.1 跨平台 UI 方案
*   **Rule**: Desktop 采用 **Tauri v2 (WebView)** 作为跨平台外壳，前端代码与 Web 端共享。
*   **Consistency**: 交互与布局规则 **MUST** 与本章一致。
*   **Note**: "原生 UI" 在此指用户体验层面（窗口管理、菜单栏、系统托盘等），而非技术实现层面。

### 4.2 内嵌服务 (Embedded Service)
*   **Rule**: 后端服务 **MUST** 内嵌并由桌面端进程拉起。
*   **Local API**: 前端与服务通信 **MUST** 走本机回环或进程内通道。

### 4.2.1 服务启动流程 (Service Boot)
*   **Rule**: Desktop App 启动 **MUST** 先拉起内嵌服务，再启动 UI。
*   **Port**: 端口 **MUST** 使用本机随机可用端口并保存在运行时内存中。
*   **Lifecycle**: 关闭主窗口 **SHOULD** 提供安全退出或后台驻留选项。
*   **Port Conflict**: 若端口占用，**MUST** 自动回退到新的可用端口并重新绑定。

### 4.2.2 本地通信策略 (Local IPC)
*   **Default**: 本机回环 HTTP/WS（`127.0.0.1`）优先。
*   **Fallback**: 若平台限制端口访问，**MUST** 提供进程内通道 (IPC) 替代方案。
*   **Security**: 本地通信 **MUST** 禁止跨进程未授权访问。
*   **Auth**: IPC **MUST** 具备进程级鉴权与会话绑定。

### 4.2.3 端口绑定安全 (Port Binding Security)
*   **Rule**: 服务端 **MUST** 仅监听 `127.0.0.1`。
*   **Firewall**: **SHOULD** 显式阻断非回环访问。

### 4.3 离线优先 (Offline-First)
*   **Rule**: 无网络时 **MUST** 保证完整编辑与索引能力。
*   **Sync**: 恢复网络后增量同步，冲突策略以本地优先。

### 4.3.1 数据持久化 (Persistence)
*   **Rule**: 所有内容 **MUST** 落盘到本地数据库与 Vault。
*   **Crash Safety**: 崩溃后 **MUST** 可恢复到最后一次持久化状态。
*   **Migration Boundary**: 桌面端 UI **MUST NOT** 自行定义存储迁移语义；涉及 Ledger / Vault Schema 的升级必须遵循 `04_storage.md` 的 `Copy & Rebuild` 策略，失败时进入显式恢复流程而不是静默自动回滚。

### 4.3.2 加密策略 (Encryption)
*   **At-Rest**: 本地存储 **MUST** 支持加密（密钥绑定设备安全模块）。
*   **In-Memory**: 解密后的明文 **SHOULD** 尽量短时保留。
*   **Key Rotation**: **MUST** 支持密钥轮换与失效，轮换过程不得破坏现有数据。
*   **Recovery**: **MUST** 提供密钥恢复策略，避免单点损坏。

### 4.3.3 备份与导出 (Backup & Export)
*   **Backup**: **MUST** 支持本地加密备份。
*   **Export**: **SHOULD** 支持单文档/全量导出。

### 4.3.4 权限与审计 (Permissions & Audit)
*   **Rule**: 本地操作 **MUST** 具备最小权限原则。
*   **Audit**: **SHOULD** 记录关键操作日志（创建/删除/导出/恢复）。

### 4.3.5 恢复演练 (Recovery Drill)
*   **Rule**: 版本升级 **SHOULD** 提供可执行的恢复演练流程。
*   **Goal**: 发生故障时可快速回退到稳定版本。

### 4.4 体积与性能约束 (Size & Performance)
*   **Size**: 体积 **MUST** 控制在可接受范围，避免 UI 框架臃肿。
*   **Perf**: 启动速度与输入延迟优先于视觉特效。

# 08_ui_design_02_desktop.md - 桌面端设计 (Desktop UI)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Counterpart Feature`: `docs/features/08_ui_design.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/05_ui.md`
- `Primary Code Areas`: `apps/web/src/components/`, `apps/web/src/hooks/use_core/`, `apps/desktop/`

本节定义了 Desktop 端的”驾驶舱”布局规范与交互逻辑。

> **原生边界**：Desktop native 是壳层与本机 service 绑定层，不拥有业务 authority。
> **目标**：Desktop 端目标采用 **Tauri v2** 外壳，前端代码与 Web 端共享。
> **离线目标**：Desktop 端目标是在无公网环境下保持本地编辑可用；完整 offline-first packaging/readiness 能力必须等 native-packaging 与 process adapter gate 打开后验收。

> **Web 映射**：当 Web 端 $W_{view} > 768px$ 时，界面 **MUST** 遵循本章 Desktop 规范。

## 0. 原生适配器边界 {#desktop-current-native-boundary}

Desktop native adapter 的边界如下：

*   Web 端大屏视口 **MUST** 映射到 Desktop 交互规范。
*   Native adapter 第一阶段只允许承担：拉起受控内嵌服务、注入本机 service endpoint/session、报告 readiness/offline 状态、转发有限平台事件。
*   默认构建 **MUST** 保持 no-Tauri skeleton；真实 `tauri` / `tauri-build` dependency 只能在 `native-packaging` feature 与独立 gate 打开后引入。
*   child-process adapter **MUST** 等 process adapter gate 显式打开后才能启动、持有或重启后端子进程。
*   recovery bootstrap 只能表达 `service_offline`、`foreground_reprobe` 与 `session_invalid` 等结构化状态；无效 endpoint 或 session-pending **MUST NOT** 退化为端口扫描。
*   Native adapter **MUST NOT** 重新定义 Ledger/Vault authority、schema migration、source-control 语义或搜索索引语义；这些仍归 core/server。
*   UI readiness **MUST** 等待内嵌服务完成 loopback/IPC endpoint 与认证会话绑定后再打开主界面；失败时显示恢复入口而不是进入半可写状态。

### 0.1 Minimal Native Adapter Contract {#desktop-native-adapter-contract}

Desktop native adapter 是进程与平台壳层，不是业务 authority。第一阶段只允许把现有
Web shell 绑定到本机受控 service，并把 runtime 状态结构化交给 Web/application
control。

Packaging dependency gate 见 `14_tech_stack.md#native-packaging-dependency-gate`。

### 0.2 Desktop Packaging Scaffold {#desktop-packaging-scaffold}

Desktop packaging scaffold 只描述首个桌面壳层批次，不等价于 packaging gate 已打开：

*   dependency batch: `tauri` + `tauri-build`，只能落在 `apps/desktop`。
*   capabilities: window shell、menu bar、system tray、installer、auto-update。
*   forbidden authorities: ledger、vault、source-control、search index、`.git` mirror、
    `.notegit`。
*   no-packaging skeleton tests 仍是 endpoint/session/bootstrap/readiness correctness 的
    authority；packaging acceptance 不得替代这些测试。

在真正添加 Tauri dependency 前，`scripts/check-native-track-boundary.sh` 必须继续阻止
Cargo dependency/import 泄漏。

### 0.2.1 Desktop Packaging Dependency Gate Decision {#desktop-packaging-dependency-gate-decision}

默认决策：Desktop packaging dependency gate **不打开**；真实 `tauri` / `tauri-build`
dependency 继续 deferred 到独立 runtime batch。

策略锚点：

*   `CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY.decision =
    DeferredUntilRuntimeBatch`
*   `real_tauri_dependencies_allowed = false`
*   `default_build_remains_no_tauri = true`
*   `native_feature_gate_required = true`
*   `authority_writes_allowed = false`

原因：

*   Desktop shell、supervisor、process-adapter decision 都仍处于 contract/skeleton 层；
    现在打开 Tauri dependency 会把窗口 runtime、菜单/托盘、安装包、auto-update 与
    service supervision 一次性耦合，回归面过大。
*   仓库仍需要保持低成本 `cargo test -p deve_desktop` default build；真实 Tauri
    dependency 应作为单独批次接受更重的平台构建、安装包与签名验收。
*   packaging scaffold 继续保留 dependency batch 规划，作为后续打开 gate 的输入；
    scaffold 不是 dependency gate 已打开的证明。

后续若打开 gate，必须先更新 `scripts/check-native-track-boundary.sh` 的允许规则，并保持：
默认构建 no-Tauri、依赖只在 `apps/desktop` feature 后、packaging 不获得
ledger/vault/source-control/search/`.git`/`.notegit` authority。

### 0.3 Embedded Service Supervisor Contract {#desktop-service-supervisor-contract}

Desktop native supervisor contract 只固定 supervisor 状态机与 failure 分类，避免后续 Tauri/process integration
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
*   Native 壳也可以注入只含 `service_state` 的 recovery bootstrap；该 payload 只能表达 `service_offline`、`foreground_reprobe` 或 `session_invalid`，不得携带 token、session secret、服务失败 reason 或 repo 写权限。
*   `?ws_port=` 只能作为开发期 fallback。native production 不得让 Web 端枚举、猜测或扫描本机端口。
*   session 绑定完成前 Web shell 不得显示可写主界面；过期 session 必须走 `09_auth.md#unauthorized-disconnected-ui`。

**Offline/readiness semantics**:

*   `NetworkOffline` 只表示公网不可用；如果 embedded service、session 与 writer gate 仍 ready，Desktop 本地编辑仍可继续。
*   `ServiceOffline` 表示本机后端不可达；UI 必须进入恢复/只读状态，不得假装仍有本地 authority。
*   App 从后台/驻留状态恢复时必须重新 probe `/api/auth/status`、`/api/node/role`，并重新确认 ws repo handshake；旧 `scope_nonce` 不得自动恢复写态。
*   `RuntimeReady` 只有在 endpoint/auth/node-role/repo-handshake/writer-ready/current-scope 全部满足时成立；`Foreground` 或 `Resumed` 事件会进入 `ForegroundReprobe`，直到 fresh readiness 完整通过。

**Forbidden native shortcuts**:

*   native 层不得直接写 ledger/vault/source-control/search index。
*   native 层不得直接操作 `.notegit/` 或 `.git/` 来伪造 source-control 成功。
*   native 层不得把平台 online/offline、窗口焦点或 Tauri lifecycle 事件解释成业务可写状态。

**Acceptance before native implementation**:

*   service bind 失败时显示恢复入口，不进入半可写 UI。
*   session invalid 时进入 `Unauthorized` 并停止普通重连。
*   network offline 但 service/session/writer ready 时，本地编辑继续可用。
*   resume/restart 后必须重新握手，stale `scope_nonce` 写入被拒绝。

### 0.4 Process Adapter Decision {#desktop-process-adapter-decision}

默认决策：真实 desktop process adapter **不进入** 默认 no-Tauri skeleton。

原因：

*   supervisor contract 必须先固定 readiness/failure/session handoff 语义；过早加入真实
    child-process runtime 会把 process lifecycle、packaging runtime、platform
    permission 与 service authority 过早耦合。
*   默认 skeleton 的工程价值是快速验证 endpoint/session/bootstrap/write-gate 边界；
    若在该层启动真实后端进程，单元测试会变成集成进程测试，失去低成本回归能力。
*   子进程 stdout/stderr、crash、signal、port reuse、profile/config/vault/ledger 参数选择
    都需要 packaging/runtime 批次统一处理；不能由 shell skeleton 临时实现。

策略锚点：

*   `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY.decision =
    DeferredUntilPackagingGate`
*   `child_process_runtime_enabled = false`
*   `packaging_gate_required = true`
*   `authority_writes_allowed = false`

后续若打开真实 process adapter，必须满足：

*   仍只落在 `apps/desktop` 的 `native-packaging` feature 后，不得进入 workspace
    root、core、cli 或 web 默认构建。
*   只允许做受控 child-process spawn、health-probe、session-handoff、restart-budget
    wiring；不得直接写 ledger/vault/source-control/search/`.git`/`.notegit`。
*   所有可写 UI 仍由 server/core 的 repo scope、writer-ready 与 `scope_nonce` 决定；
    process running 不等于 writable。

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

本节是 **post-gate normative target**：只有当 `native-packaging` 与 process adapter gate
被显式打开后，以下 Tauri/embedded-service 规则才进入验收；默认 no-Tauri skeleton
仍以 §0 的原生适配器边界为准。

*   **Rule**: Desktop post-gate 采用 **Tauri v2 (WebView)** 作为跨平台外壳，前端代码与 Web 端共享。
*   **Consistency**: 交互与布局规则 **MUST** 与本章一致。
*   **Note**: "原生 UI" 在此指用户体验层面（窗口管理、菜单栏、系统托盘等），而非技术实现层面。

### 4.2 内嵌服务 (Embedded Service)
*   **Rule**: post-gate 后端服务 **MUST** 内嵌并由桌面端进程拉起。
*   **Local API**: 前端与服务通信 **MUST** 走本机回环或进程内通道。

### 4.2.1 服务启动流程 (Service Boot)
*   **Rule**: post-gate Desktop App 启动 **MUST** 先拉起内嵌服务，再启动 UI。
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
*   **Rule**: post-gate 无公网时 **MUST** 保证本地编辑能力；完整本地索引能力仍受 profile、search feature 与资源预算约束。
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

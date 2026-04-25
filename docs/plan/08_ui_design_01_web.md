# 08_ui_design_01_web.md - Web 端设计 (Web UI)

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Counterpart Feature`: `docs/features/08_ui_design.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/05_ui.md`
- `Primary Code Areas`: `apps/web/src/components/`, `apps/cli/src/server/static_files.rs`

本节定义了 Web 端作为 **Server Dashboard + WebLightPeer Thin Client** 的特有功能与部署架构。

> **Scope Boundary**: Web 端承担服务器侧 UI 与浏览器薄客户端写入界面，但不承担 Native 端完整离线能力。移动端/桌面端 **MUST** 采用 **Tauri v2 (原生外壳 + 内嵌 WebView)** 方案，提供原生级体验 (Native-feel)。详见 `08_ui_design_02_desktop.md` §4.1 和 `08_ui_design_03_mobile.md` §7.1。

## 1. Normative Language (规范性用语)
*   **MUST**: 绝对要求。
*   **SHOULD**: 强烈建议。

## 2. Single Binary Distribution (部署架构) {#single-binary-distribution}

为了实现“零依赖部署”，CLI 二进制文件 **MUST** 内嵌前端静态资源。

### 1.1 构建流水线 (Build Pipeline)
定义构建依赖链 $Build_{full} = Build_{web} \to Embed \to Build_{cli}$：

1.  **Web Build**: `trunk build --release` 生成 `apps/web/dist`。
2.  **Embed**: CLI 通过 `rust-embed` 宏读取 `dist` 目录。
3.  **CLI Build**: `cargo build --release` 生成最终可执行文件。

### 1.2 路由回退 (SPA Routing)
后端服务器 **MUST** 实现 SPA 路由回退逻辑：
$$ \forall path \notin API, Serve(path) \to index.html $$
这确保了前端路由刷新时不会 404。

### 2.1 Viewport Mapping (视口适配策略)

*   **Rule**: Web 端 **MUST** 根据视口宽度映射到 Mobile / Desktop 规范。
*   **Mobile View**: $W_{view} \le 768px$ 时，Web UI **MUST** 与 Mobile UI 规范一致。
*   **Desktop View**: $W_{view} > 768px$ 时，Web UI **MUST** 与 Desktop UI 规范一致。

## 3. Server Dashboard

### 2.1 仪表盘布局 (Dashboard Layout)
当访问根路径 `/` 且无特定文档 ID 时，显示系统概览。

**Metrics Visualization**:

| Card | Content Description | Refresh Policy |
| :--- | :--- | :--- |
| **System Health** | CPU Load, RAM Usage, Uptime | Polling (5s) |
| **Sync Status** | Connected Peers, Ops Queue | Push (WS) |
| **Storage Stats** | DB Size, Document Count | On Load |
| **Actions** | `[New Doc]` `[Sync Now]` | Interactive |

### 2.2 数据协议 (Data Protocol)
前端与后端通过 WebSocket 交换 `SystemMetrics` 结构体：

```rust
struct SystemMetrics {
    cpu_usage_percent: f32,
    memory_used_mb: u64,
    active_connections: u32,
    ops_processed: u64,
}
```

### 2.3 安全约束 (Safety Constraints)
*   **Disconnect Lockdown**: 当网络断连时，UI **MUST** 立即被遮罩层锁定，禁止任何写操作，并显示重连状态。
*   **Session Expiry Split**: 当 user session 失效或鉴权被拒绝时，UI **MUST NOT** 继续显示无限重连遮罩；必须切换到明确的登录失效 / 未认证状态。
*   **RAM-Only**: Dashboard 数据 **MUST NOT** 持久化到 IndexedDB。

### 2.4 Dashboard 路由与权限
*   **Route**: `/` (根路径，无 DocId 参数时)。
*   **Auth**: Dashboard **MUST** 要求已认证身份。未认证访问跳转 Login。
*   **Data Channel**: 通过现有 WebSocket 连接推送 `ServerMessage::SystemMetrics`。
*   **Fallback**: 网络断连时，Metrics 冻结并显示 `Disconnected`；session 失效时，直接退出到登录页或认证失效界面。

## 4. External Edit Flow

专门针对“用户在服务器端直接修改文件”的场景；其职责是暴露工作区差异，而不是直接改写已确认编辑状态。

1.  **Detection**: 后端 `notify` 监听到文件系统变更 $Event_{fs}$。
2.  **Record**: 变更经 Debouncer 与路径归一化后写入 repo-scoped `pending_fs_ops`，**MUST NOT** 直接入 Ledger。
3.  **Push**: 后端通过 `FsChangeDetected` 提示前端刷新当前 repo 的 Changes / Staging 视图。
4.  **Feedback**: 若当前文档受影响，前端显示“磁盘上检测到未确认变更”的可感知提示，但 **MUST NOT** 直接用外部文件内容覆盖编辑器中的 confirmed + pending overlay。

## 5. PWA Support
Web 端 **SHOULD** 提供 `manifest.json` 以支持安装到主屏幕：
*   `display`: `standalone` (隐藏浏览器 UI)。
*   `theme_color`: `#1e1e1e` (匹配 Dark Mode)。

## 6. Resizable Layout

*   **Scope**: 左侧 Sidebar 与主编辑区之间、主编辑区与右侧面板之间。
*   **Constraints**:
    *   Sidebar Width: `180px` ~ `500px`。
    *   Right Panel Width: `240px` ~ `520px`。
*   **Persistence**: 伸缩宽度 **MUST** 通过 `localStorage` 持久化。
*   **Outer Gutter**: 主区域左右边距 **MUST** 支持拖拽调整，并持久化。

### 6.1 Sync State Presentation

Web 端除 Dashboard 指标外，还必须明确呈现 WebLightPeer 的同步能力、降级状态与 repo-scoped peer 行为。

*   **Connection Indicator**:
    *   绿色：`Connected + Synced`，表示当前 repo 的 peer identity 已完成握手且 vector 最新。
    *   黄色：`Reconnecting`，显示退避重试中与最近一次失败原因。
    *   灰色：`Read-only`，表示进入 `DegradedSyncMode` 或尚未完成 peer registration。
*   **Status Copy**:
    *   UI **MUST** 同时区分 `session token` 状态与 `peer identity` 状态，例如“已登录 / Peer 未注册”。
    *   UI **MUST** 同时区分 `network disconnected` 与 `session expired`；前者允许重连，后者要求重新登录。
    *   Repo 切换时 **MUST** 显示 `Handshaking repo...`，直到新的 repo-scoped peer identity 完成注册。
*   **DegradedSyncMode Banner**:
    *   当 IndexedDB 或 WebCrypto 不可用时，顶部 **MUST** 显示只读横幅，明确原因是浏览器持久存储不可用。
    *   横幅文案 **SHOULD** 说明：允许查看与拉取，禁止编辑提交与 `SyncPush`。
*   **Editing Guardrails**:
    *   只读模式下编辑器、创建按钮、同步推送按钮 **MUST** 禁用或隐藏。
    *   若仅 user session 有效但 peer identity 缺失，UI **MUST** 允许重试注册，而不是静默失败。
*   **Repo Switch Flow**:
    *   用户切换 repo 时，旧 repo 的同步状态立即清空，新 repo 进入握手态。
    *   若新 repo 无法恢复 IndexedDB identity，则 UI 显示“临时只读 peer”并阻止写入，直到注册成功。
    *   浏览器刷新后 **SHOULD** 恢复最近一次稳定的 `repo_name + repo_id + active_branch` 组合，但实际绑定 **MUST** 以 `repo_id` 为准，名称仅作展示或辅助恢复。
    *   `SwitchRepo / SwitchBranch` 发出的 `switch_nonce` **MUST** 严格大于当前 `scope_nonce`，避免旧 scope 的迟到消息污染新 scope。

### 6.2 Web Shell Interaction Rules

*   **Activity Bar More...**:
    *   菜单项整行点击 **MUST** 执行“切换视图”。
    *   `Pin/Unpin` **MUST** 是独立操作，不得与视图切换复用同一点击语义。
*   **Repo Switcher**:
    *   触发器与菜单项 **SHOULD** 使用按钮语义，并支持点击外部自动收起。
*   **Open in New Window**:
    *   新窗口链接 **MUST** 保留现有 query params，并在其上正确追加 `doc=...`，不得生成重复 `?` 或破坏现有 URL 状态。

## 7. Implementation Boundaries

*   **Rule**: Web 端承载 Dashboard 与 thin-client 写入界面，但不承载移动/桌面端原生实现细节。
*   **Offline**: Web 端离线能力仅限 PWA 缓存，**MUST NOT** 替代内嵌服务。

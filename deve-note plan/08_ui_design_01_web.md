# 08_ui_design_01_web.md - Web 端设计 (Web UI)

> **Status**: Core Specification
> **Target**: Server Dashboard & PWA

本节定义了 Web 端作为 Server Dashboard 的特有功能与部署架构。

> **Scope Boundary**: Web 端仅作为服务器侧 UI。移动端/桌面端 **MUST** 采用原生 UI + 内嵌服务。

## 规范性用语 (Normative Language)
*   **MUST**: 绝对要求。
*   **SHOULD**: 强烈建议。

## 1. 部署架构：单二进制分发 (Single Binary Distribution)

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

## 1.3 视口适配策略 (Viewport Mapping)

*   **Rule**: Web 端 **MUST** 根据视口宽度映射到 Mobile / Desktop 规范。
*   **Mobile View**: $W_{view} \le 768px$ 时，Web UI **MUST** 与 Mobile UI 规范一致。
*   **Desktop View**: $W_{view} > 768px$ 时，Web UI **MUST** 与 Desktop UI 规范一致。

## 2. 服务器仪表盘 (Server Dashboard)

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
*   **Disconnect Lockdown**: 当 WebSocket 断开时，UI **MUST** 立即被遮罩层锁定，禁止任何写操作，并显示 "Reconnecting..."。
*   **RAM-Only**: Dashboard 数据 **MUST NOT** 持久化到 IndexedDB。

## 3. 外部协同流程 (External Edit Flow)

专门针对“用户在服务器端直接修改文件”的场景。

1.  **Detection**: 后端 `notify` 监听到文件系统变更 $Event_{fs}$。
2.  **Push**: 后端生成 `ExternalChange` 操作并通过 WS 推送。
3.  **Merge**: 前端接收后，通过 CRDT 或简单的 "Last Writer Wins" 策略更新编辑器内容。
4.  **Feedback**: 弹出 Toast 提示 "File updated on disk"。

## 4. PWA 支持
Web 端 **SHOULD** 提供 `manifest.json` 以支持安装到主屏幕：
*   `display`: `standalone` (隐藏浏览器 UI)。
*   `theme_color`: `#1e1e1e` (匹配 Dark Mode)。

## 5. 布局伸缩 (Resizable Layout)

*   **Scope**: 左侧 Sidebar 与主编辑区之间、主编辑区与右侧面板之间。
*   **Constraints**:
    *   Sidebar Width: `180px` ~ `500px`。
    *   Right Panel Width: `240px` ~ `520px`。
*   **Persistence**: 伸缩宽度 **MUST** 通过 `localStorage` 持久化。
*   **Outer Gutter**: 主区域左右边距 **MUST** 支持拖拽调整，并持久化。

## 6. 实现策略边界 (Implementation Boundaries)

*   **Rule**: Web 端仅作为服务器 UI，不承载移动/桌面端原生实现细节。
*   **Offline**: Web 端离线能力仅限 PWA 缓存，**MUST NOT** 替代内嵌服务。

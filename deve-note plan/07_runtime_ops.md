# 第七章：双引擎插件运行时 (Dual-Engine Plugin Runtime)

### 1. Engine A: Wasm Runtime (轻量引擎)
*   **适用场景**：UI 组件扩展、字符串处理、简单的逻辑控制。
*   **技术栈**：**Rhai** (脚本) 或 **Extism** (Wasm)。
*   **可用性**：所有端 (Desktop / Mobile / Server / Web)。
*   **约束**：严格沙箱 (内存/指令限制)。由于 Web 端无法直接运行 Podman，所有计算请求需转发。

### 2. Engine B: Podman Runtime (计算引擎)
*   **适用场景**：**可复现的科学计算**、Python/R 代码块。
*   **技术栈**：**Podman (Rootless)** + **OCI Containers**。
*   **可用性**：仅 **Desktop** 和 **Server**。
    *   **Web 端行为**：Web 端请求执行代码块时，通过 WebSocket 转发给 **Server** 执行。
*   **工作流**：Frontmatter 定义 runtime -> Run -> 启动临时容器 -> 注入代码 -> 捕获输出 -> 销毁。
*   **安全**：Rootless, No Net (默认), Read-only Volume (默认)。

### 3. 通用插件协议
*   **ABI**：Manifest -> Install/Activate -> Events.
*   **Host Functions**：受控 API，必须 Capability 校验。
*   **RPC Bridge**：前端 `client.call` -> WebSocket -> 后端插件。
*   **资源配额**：CPU/Mem/Timeout 可配。

### 可执行代码块扩展
*   识别 `fenced block`，路由至对应 Runtime 执行。

# 第八章：AI 与计算扩展
*   **AI 抽象层**：Provider-agnostic 接口。
*   **安全**：访问网络/文件必须显式授权。
*   **隐私**：默认关闭遥测。

# 第九章：多端发布与封装策略 (Cross-Platform Delivery)
*   **单内核，多外壳**：Rust Core + Leptos UI。
*   **外壳适配**：
    *   Desktop: Tauri v2。
    *   Web: Wasm + PWA。
    *   Mobile: Tauri Mobile / WebView。
*   **存储适配**：
    *   Desktop: Redb (Disk)。
    *   Mobile: Redb (Recommended) / SQLite. (No IndexedDB, Full Peer).
    *   **Web (Dashboard)**: **RAM-Only**. No IndexedDB.
*   **计算适配**：Desktop/Server 支持 Podman；Web/Mobile 需转发。

# 第十章：开源发布与社区运营
*   **许可证**：MIT / Apache-2.0。
*   **发行物**：Tauri bundle, Docker Image, PWA.
*   **GitHub Releases**：提供二进制与校验和。
*   **Docker 镜像**：GHCR, 支持数据卷挂载。
*   **插件发布**：官方索引，支持离线安装。

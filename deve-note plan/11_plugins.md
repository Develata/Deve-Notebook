# 11_plugins.md - 插件篇 (Plugins)

## 双引擎插件运行时 (Dual-Engine Plugin Runtime)

### 1. Engine A: Wasm Runtime (轻量引擎)
*   **技术栈**：**Rhai** (脚本) 或 **Extism** (Wasm)。
*   **适用场景**：UI 组件扩展、字符串处理。
*   **约束**：严格沙箱。

### 2. Podman/Docker (计算引擎)
*   **适用场景**：**可复现的科学计算**、Python/R 代码块。
*   **技术栈**：**Podman (Rootless)** + **OCI Containers**。
*   **Web 端行为**：Web 端请求执行代码块时，通过 WebSocket 转发给 **Server** 执行。
*   **安全 (Security Focus)**：
    *   **Anti-Injection (防注入)**: 严禁直接 `eval`。所有代码执行**必须**通过外置插件 (`Podman`) 在隔离容器中运行。
    *   **Rootless**: 默认非特权模式运行。
    *   **Network**: 默认无网络访问 (No Net)。
    *   **Volume**: 默认只读挂载 (Read-only Volume)。
*   **Workflow (工作流)**: `Frontmatter defined runtime` -> `Run` -> `Start Ephemeral Container` -> `Inject Code` -> `Capture Output` -> `Destroy`.

### 3. 通用插件协议 (Plugin Protocol)
*   **ABI Lifecycle**: Manifest -> Install -> Activate -> Events.
*   **Host Functions**: 受控 API，必须 Capability 校验 (default deny)。
*   **RPC Bridge**: 前端 `client.call` -> WebSocket -> 后端插件。
*   **Resource Quotas**: CPU/Mem/Timeout 可配。

### 4. AI 辅助 (AI Assistance)
*   **AI 抽象层**：Provider-agnostic 接口，允许插件注入不同模型提供商。
*   **Chat 界面**：支持在 Side Bar 注入 Copilot Style 的对话界面。
*   **安全**：访问网络/文件必须显式授权 Capability。
*   **隐私 (Privacy)**：默认关闭遥测 (Telemetry Off by Default).

### 5. Git 推送 (Git Integration)
*   **机制**：调用 Host Functions 中的 `git_sync.rhai`。
*   **流程**：`Frontend -> Command/Button -> Check Capability -> Host Function -> git add/commit/push -> Feedback`。
*   **真正的 CLI**：在受控环境下调用系统 `git` 命令。

### 6. LaTeX (数学引擎)
*   集成 **KaTeX** 或 **MathJax** 引擎，支持高性能数学公式渲染。
*   作为核心体验的一部分，但也设计为可替换/可升级的插件化模块。

## 本章相关命令

*   `Git: Sync`: 同步 (Pull & Push).
*   `Git: Commit`: 提交更改.
*   `Git: Push`: 推送至远程.

## 本章相关配置

*   `plugin.podman.path`: Podman 可执行文件路径.
*   `ai.provider`: AI 服务提供商 (e.g., `openai`, `anthropic`).

# 11_plugins.md - 插件篇 (Plugins)

## 双引擎插件运行时 (Dual-Engine Plugin Runtime)

### 1. Engine A: Application Runtime (轻量级/嵌入式)
此层级的插件直接运行在宿主进程内（或其 Webview 中），负责 UI 扩展和数据处理。
*   **Performance Constraint (性能约束)**:
    *   WASM 虽然高效，但频繁跨越边界操作 DOM (WASM <-> JS) 会带来巨大的序列化/反序列化开销。
    *   因此，**禁止** WASM 插件直接进行细粒度的 DOM 操作 (e.g., 单个字符的样式渲染)。
*   **Hybrid Architecture (混合架构)**:
    *   **Logic Layer (WASM/Rust)**: 负责重型计算、数据清洗、Linter 规则校验、AI 上下文组装。输入为纯文本，输出为结构化数据 (JSON)。
    *   **UI Layer (JS/JSON Protocol)**: 前端宿主负责解析 WASM 输出的 "Rendering Instructions" (渲染指令)，并通过原生 JS 高效更新 DOM。
    *   **Example**: 自定义 Linter 插件 -> WASM 计算出 `[ { line: 10, msg: "Error" } ]` -> JS 接收并调用 CodeMirror API 绘制波浪线。

*   **Dual-Layer Strategy (双层架构)**:
    1.  **Scripting Layer (Rhai)**:
        *   **用途**: 轻量逻辑 (e.g., 自定义日期格式化, 简单的保存钩子).
        *   **优势**: 零编译，直接修改脚本即可生效，Rust 原生嵌入。
    2.  **Binary Layer (WASM / Extism)**:
        *   **用途**: 重型逻辑 (e.g., 自定义 Linter, AI Agent SDK).
        *   **优势**: 高性能，多语言支持 (Rust/Go/JS -> WASM)，强沙箱隔离。

### 2. Engine B: Calculation Runtime (计算引擎)
此层级用于运行不可信的、需要完整 OS 环境的代码块 (e.g., Python Notebook, R).
*   **核心技术**: **Podman (Rootless)** + **OCI Containers**.
*   **Web 端行为**: Web 前端无法直接通过 WASM 调用 Podman，**MUST** 通过 WebSocket 请求后端完成执行 (Remote Execution)。
*   **Security (安全沙箱)**:
    *   **No Root**: 强制使用 Rootless 容器。
    *   **No Net**: 默认禁止网络，除非用户显式授权。
    *   **Ephemeral**: 用完即焚 (One-off containers)。

### 3. 通用插件协议 (Plugin Protocol)
*   **ABI Lifecycle**: Manifest -> Install -> Activate -> Events.
*   **Manifest (清单)**: 结构体位于 `crates/core/src/plugin/manifest.rs`.
    *   Fields: `id`, `name`, `version`, `entry` (脚本入口路径).
    *   **Capabilities (权限能力)**:
        *   `allow_net`: 域名白名单 (精确匹配).
        *   `allow_fs_read` / `allow_fs_write`: 路径白名单 (前缀匹配, 自动标准化).
        *   `allow_env`: 环境变量白名单.
*   **Host Functions**: 受控 API，必须 Capability 校验 (default deny)。
*   **RPC Bridge**: 前端 `client.call` -> WebSocket -> 后端插件。
*   **Resource Quotas**: CPU/Mem/Timeout 可配。

### 4. AI Integration (AI 插件)
系统预留了专门的 `AI Chat Slot` (UI Column 5)，但不内置任何具体模型，全靠插件驱动。
*   **Provider Independent**: 官方提供标准 `AI_Provider_Trait` (WASM ABI)，社区可开发 `OpenAI Plugin`, `Ollama Plugin` 等。
*   **Context Safety**: 插件读取用户选中的代码上下文 **MUST** 经过用户确认 (或配置白名单)。
*   **Chat 界面**: 插件通过标准协议向 UI 渲染 Markdown 消息流。
*   **安全**：访问网络/文件必须显式授权 Capability。
*   **隐私 (Privacy)**：默认关闭遥测 (Telemetry Off by Default).

### 5. Git 推送 (Git Integration)
*   **机制**：调用 Host Functions 中的 `git_sync.rhai`。
*   **流程**：`Frontend -> Command/Button -> Check Capability -> Host Function -> git add/commit/push -> Feedback`。
*   **真正的 CLI**：在受控环境下调用系统 `git` 命令。

### 6. LaTeX & Extensions (数学引擎与扩展)
*   **Core Engine**: 集成 **KaTeX** 引擎 (v0.16+)，支持高性能数学公式渲染 (Inline/Block).
*   **Extensions (扩展库)**:
    *   **Need**: 虽然核心库已集成，但高级功能需动态加载扩展模块。
    *   **List**:
        *   `mhchem.js`: 化学方程式支持 (`\ce{H2O}`).
    *   **Implementation**: 作为内置 "System Extensions" 存在，默认不加载，用户通过 `config.tex_extensions` 启用以减少包体积。

## 本章相关命令

*   `Git: Sync`: 同步 (Pull & Push).
*   `Git: Commit`: 提交更改.
*   `Git: Push`: 推送至远程.

## 本章相关配置

*   `plugin.podman.path`: Podman 可执行文件路径.
*   `ai.provider`: AI 服务提供商 (e.g., `openai`, `anthropic`).

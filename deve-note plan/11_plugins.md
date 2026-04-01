# 11_plugins.md - 插件篇 (Plugins)（目前只需要制作接口，不需要具体实现）

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
    *   **Capabilities (权限能力)** — Default Deny，插件仅获得声明的权限:
        *   `allow_net`: `Vec<String>` — 网络域名白名单 (精确匹配)。
        *   `allow_fs_read` / `allow_fs_write`: `Vec<PathBuf>` — 文件路径白名单 (前缀匹配, 自动标准化, 防遍历)。
        *   `allow_env`: `Vec<String>` — 环境变量白名单。
        *   `allow_source_control`: `bool` — 是否允许 Git 操作。
        *   `allow_search`: `bool` — 是否允许使用 glob/grep 搜索 API。
        *   `allow_skill`: `bool` — 是否允许读取 Skill 列表与内容。
        *   `allow_mcp`: `bool` — 是否允许调用 MCP 工具。
        *   `allow_project_tree`: `bool` — 是否允许获取项目目录树。
*   **Ledger-Managed Boundary (账本托管边界)**:
    *   `allow_fs_write` **不是**“允许插件直接修改托管笔记”的总开关；它只授予对白名单路径的**原始文件写入尝试资格**。
    *   插件运行时 **MUST NOT** 通过 `fs_write` 直接写入任何 **Ledger-Managed Projection Objects**：
        *   `vault/<repo>/**/*.md`
        *   `vault/<repo>/.notegit/**`
        *   `ledger/**`
    *   上述对象的规范状态由 `Ledger Facts -> Projection -> Vault` 决定；即使路径同时命中 `allow_fs_write` 白名单，也 **MUST** 被拒绝。
    *   `fs_write` 允许的目标应限于 **Non-Ledger Assets**，例如导出文件、缓存、附件、临时产物或用户显式声明的普通输出目录。
    *   若插件希望修改托管笔记内容，**MUST** 走 ledger-aware host functions（如未来的 note/source-control API），而不是原始文件 I/O。
*   **Host Functions**: 受控 API，必须 Capability 校验 (default deny)。
*   **RPC Bridge**: 前端 `client.call` -> WebSocket -> 后端插件。
*   **Resource Quotas**:
    *   **Rhai**: `max_operations = 100,000` — 防止无限循环；`max_expr_depths = 128` — 防止栈溢出。
    *   **WASM/Podman**: CPU/Mem/Timeout 可配。

### 4. Git 推送 (Git Integration)
*   **机制**：调用 Host Functions 中的 `git_sync.rhai`。
*   **流程**：`Frontend -> Command/Button -> Check Capability -> Host Function -> git add/commit/push -> Feedback`。
*   **真正的 CLI**：在受控环境下调用系统 `git` 命令。

### 5. LaTeX & Extensions (数学引擎与扩展)
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

# AI 双通道架构 (Dual-Channel AI)

> **两条路径均为第一方扩展能力（first-party bundled extension）**，不是 `Core MUST`。
> 桥接层本身极轻量 (< 5 MB)，不违背低资源原则。
> 真正占用内存的是外部 CLI 进程，且按需启动、用完即销毁。

| 通道 | 定位 | 运行时开销 | 适用场景 |
|------|------|-----------|----------|
| **Agent Bridge** (默认) | 外部 CLI 子进程桥接 | 桥接层 ~0 MB 常驻；CLI 进程按需 15-100 MB | 需要完整 Agent 能力 (MCP/Tools/History) |
| **AI Chat** (备选) | 内置 Rhai 轻量插件 | Rhai Engine ~2-4 MB；SSE 连接按需 | 用户自带 API Key 的简单问答 + 基础工具 |

## 1. 目标与范围

*   **目标**：在低资源环境 (768MB) 中提供灵活的 AI 助手，兼顾**功能深度**与**零门槛接入**。
*   **范围**：
    - **Agent Bridge**：Deve-Notebook 作为 UI 层，复杂 Agent 逻辑全权交由外部 CLI (opencode/zeroclaw)。
    - **AI Chat**：内置轻量 Rhai 插件，直连 OpenAI 兼容 API，提供基础对话 + 文件读取 + Git 操作，无需安装外部工具。

## 2. 外部 CLI 桥接架构

### Backend Bridge (核心机制)
1. **Frontend**: 用户在 Column 5 (AI Chat Slot) 输入自然语言。
2. **WebSocket**: 指令发送至 Rust 后盾。
3. **Subprocess (`tokio::process::Command`)**:
   - 后端根据配置（如 `config.toml` 或 `.env` 中的 `AGENT_CLI_PATH`），直接起一个子进程。
   - 例如：`zeroclaw "用户的输入内容"` 或者 `opencode "..."`
4. **Streaming**: 将子进程的 `stdout` 和 `stderr` 通过 WebSocket 实时 Push 给前端，实现字级的打字机效果。

### On-Demand 内存策略
*   在非对话时段，AI Agent **占用零内存**。
*   只有当执行指令时才唤起进程，执行完毕后进程立即退出回收，完美契合 768MB 运行环境的限制。
*   相比于在 Node.js 中常驻服务器，使用 Rust 编写的外部 CLI（如 `zeroclaw`）具有极低的启动延迟和内存指纹。

## 3. Agent Bridge：为什么选择成熟 CLI？

*   **历史管理**：内置 sqlite/json 历史状态机，支持 `/plan` 和 `/build` 等模式。
*   **工具支持 (Tools & MCP)**：继承庞大的内置工具链（读写文件、Bash 执行、MCP 连接）。
*   **Skills (自定义技能)**：原生支持 `.opencode/skills/` 目录加载预设 Prompts。
*   **Token 优化**：滑动窗口和上下文合并已经做到极致。
*   **外部 CLI 内存参考**：opencode ~50-100 MB/次，zeroclaw ~15-30 MB/次，均为按需启动。

## 4. AI Chat：轻量备选方案

内置 Rhai 插件 (`plugins/ai-chat/`)，用户自带 API Key 即可使用，无需安装外部工具。

*   **功能边界 (Minimal Scope)**：
    - 单轮/多轮对话 (OpenAI 兼容 SSE 流式)
    - 基础工具调用：read_file, git_status/diff/add/commit (最多 3 轮 tool loop)
    - 系统提示词注入当前编辑文件上下文
*   **不做的事 (Out of Scope)**：
    - 不做 MCP 集成、不做历史持久化、不做复杂 Agent 状态机
    - 不做 Token 滑动窗口优化
*   **资源开销**：Rhai Engine ~2-4 MB，脚本 ~290 行，零额外 crate 依赖
*   **配置**：环境变量 `AI_API_KEY` / `AI_BASE_URL` / `AI_MODEL`

## 5. 交互与 UI

UI 层统一，两条通道共享同一套前端组件：
*   Settings 面板切换 CLI / API 模式 (`ai_mode` signal)
*   移动端折叠 Chat Sheet
*   Markdown 渲染 + 代码块横向滚动
*   错误捕获：CLI 非 0 退出码 / API 网络错误 → 前端展示重试态

## 6. 资源开销汇总

| 组件 | 代码量 | 常驻内存 | 按需内存 |
|------|--------|---------|----------|
| agent_bridge.rs | 167 行 Rust | 0 MB | pipe buffer ~64 KB |
| plugin handler | 85 行 Rust | 0 MB | — |
| ai-chat Rhai 脚本 | 290 行 Rhai | 0 MB (未加载时) | Rhai Engine ~2-4 MB |
| opencode (外部) | — | 0 MB | ~50-100 MB/次 |
| zeroclaw (外部) | — | 0 MB | ~15-30 MB/次 |

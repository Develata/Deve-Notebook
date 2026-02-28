# 11b_ai_integration.md - AI 集成篇 (AI Integration)

> AI 双通道架构为**原生功能**，桥接层极轻量 (< 5 MB)，不违背低资源原则。
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

## 2. 通道 A — Agent Bridge (默认)

### Backend Bridge (核心机制)
1. **Frontend**: 用户在 Column 5 (AI Chat Slot) 输入自然语言。
2. **WebSocket**: 指令发送至 Rust 后端。
3. **Subprocess (`tokio::process::Command`)**:
   - 后端根据配置（`AGENT_CLI_PATH` 环境变量），直接起子进程。
   - 例：`zeroclaw "用户的输入内容"` 或 `opencode "..."`
4. **Streaming**: 将子进程 `stdout` 通过 WebSocket 实时 Push 给前端，实现打字机效果。

### On-Demand 内存策略
*   非对话时段 AI Agent **占用零内存**。
*   执行时唤起进程，完毕后立即退出回收，契合 768MB 运行环境。
*   Rust 编写的外部 CLI（如 `zeroclaw`）具有极低启动延迟和内存指纹。

### 为什么选择成熟 CLI？
*   **历史管理**：内置 sqlite/json 历史状态机，支持 `/plan` 和 `/build` 等模式。
*   **工具支持 (Tools & MCP)**：继承庞大的内置工具链（读写文件、Bash 执行、MCP 连接）。
*   **Skills (自定义技能)**：原生支持 `.opencode/skills/` 目录加载预设 Prompts。
*   **Token 优化**：滑动窗口和上下文合并已做到极致。
*   **内存参考**：opencode ~50-100 MB/次，zeroclaw ~15-30 MB/次，均按需启动。

## 3. 通道 B — AI Chat (备选)

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

## 4. 前端交互 (统一 UI)

两条通道共享同一套前端组件：
*   Settings 面板切换 CLI / API 模式 (`ai_mode` signal)
*   移动端折叠 Chat Sheet
*   Markdown 渲染 + 代码块横向滚动
*   错误捕获：CLI 非 0 退出码 / API 网络错误 → 前端展示重试态

## 5. 资源开销汇总

| 组件 | 代码量 | 常驻内存 | 按需内存 |
|------|--------|---------|----------|
| agent_bridge.rs | 167 行 Rust | 0 MB | pipe buffer ~64 KB |
| plugin handler | 85 行 Rust | 0 MB | — |
| ai-chat Rhai 脚本 | 290 行 Rhai | 0 MB (未加载时) | Rhai Engine ~2-4 MB |
| opencode (外部) | — | 0 MB | ~50-100 MB/次 |
| zeroclaw (外部) | — | 0 MB | ~15-30 MB/次 |

## 本章相关配置

*   `AGENT_CLI_PATH`: 外部 CLI 路径 (默认 `opencode`)。
*   `AI_API_KEY` / `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`: AI 服务密钥。
*   `AI_BASE_URL`: API 端点 (默认 `https://api.openai.com/v1`)。
*   `AI_MODEL`: 模型名 (默认 `gpt-4o-mini`)。
*   `AI_MAX_TOKENS`: 最大 Token 数 (默认 `4096`)。

## 详细设计

*   **[Agent Bridge 详细设计](./plugins/agent_bridge/01_agent_bridge.md)**: 完整架构说明与资源估算。

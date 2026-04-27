# AI 双通道架构 (Dual-Channel AI)

> **Status (2026-04-25)**: 本文是历史设计注，当前权威边界以 `docs/plan/10_ai_agent.md` 与 `docs/plan/17_plugins.md` 为准。
> Native AI Chat 是当前默认第一方能力；Trusted External Agent Bridge 是 default-off / trusted-only 高级部署位。
> 仓库中保留的 Rhai `ai-chat` 与 `PluginCall` 路径是兼容实现细节，不代表插件平台或外部 Agent 默认启用。

| 通道 | 定位 | 运行时开销 | 适用场景 |
|------|------|-----------|----------|
| **Native AI Chat** (默认) | 第一方聊天能力，可暂由 bundled Rhai 兼容路径承载 | Rhai Engine ~2-4 MB；SSE 连接按需 | 用户自带 API Key 的简单问答 + 当前 Markdown / 显式上下文 |
| **Trusted Agent Bridge** (默认关闭) | 外部 CLI 子进程桥接 | 桥接层 ~0 MB 常驻；CLI 进程按需 15-100 MB | 受信任单用户部署中的高级 Agent 能力 |

## 1. 目标与范围

*   **目标**：在低资源环境 (768MB) 中提供灵活的 AI 助手，同时避免默认扩大工具权限。
*   **范围**：
    - **Native AI Chat**：Deve-Notebook 提供第一方聊天 UI 与 OpenAI-compatible SSE runtime。
    - **Trusted Agent Bridge**：只在 enabled + trusted + explicit executable path 条件满足时桥接受信任外部 CLI。

## 2. 外部 CLI 桥接架构

### Backend Bridge (核心机制)
1. **Frontend**: 用户在 Column 5 (AI Chat Slot) 输入自然语言。
2. **WebSocket**: 指令发送至 Rust 后盾。
3. **Subprocess (`tokio::process::Command`)**:
   - 后端只能在 trusted policy 通过后读取显式 `AGENT_CLI_PATH`。
   - `AGENT_CLI_PATH` 必须是受控路径；不得退化为任意 PATH 搜索。
4. **Streaming**: 将子进程的 `stdout` 和 `stderr` 通过 WebSocket 实时 Push 给前端，实现字级的打字机效果。

### On-Demand 内存策略
*   在非对话时段，AI Agent **占用零内存**。
*   只有当执行指令时才唤起进程，执行完毕后进程立即退出回收，完美契合 768MB 运行环境的限制。
*   相比于在 Node.js 中常驻服务器，使用 Rust 编写的外部 CLI（如 `zeroclaw`）具有极低的启动延迟和内存指纹。

## 3. Trusted Agent Bridge：为什么保留接口位？

*   **历史管理**：内置 sqlite/json 历史状态机，支持 `/plan` 和 `/build` 等模式。
*   **工具支持 (Skills + controlled CLI)**：只适用于用户明确信任外部 CLI 的部署，不是 Native AI 的默认能力；MCP 不再作为产品运行时方向。
*   **Skills (自定义技能)**：只能由外部 CLI 自己管理，Deve-Notebook 当前不内建 Skills 装载。
*   **Token 优化**：滑动窗口和上下文合并已经做到极致。
*   **外部 CLI 内存参考**：opencode ~50-100 MB/次，zeroclaw ~15-30 MB/次，均为按需启动。

## 4. Native AI Chat：默认方案

Native AI Chat 可暂由内置 Rhai 插件 (`plugins/ai-chat/`) 承载实现，但产品语义仍属于第 10 章第一方能力。

*   **功能边界 (Minimal Scope)**：
    - 单轮/多轮对话 (OpenAI 兼容 SSE 流式)
    - 读取当前 Markdown / 显式选择上下文
    - 系统提示词注入当前编辑文件上下文
*   **不做的事 (Out of Scope)**：
    - 不实现 MCP runtime，不默认启用 Skills / source-control 写入
    - 不做历史持久化、不做复杂 Agent 状态机
    - 不做 Token 滑动窗口优化
*   **资源开销**：Rhai Engine ~2-4 MB，脚本轻量，零额外 crate 依赖
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
| ai-chat Rhai 脚本 | 轻量 Rhai | 0 MB (未加载时) | Rhai Engine ~2-4 MB |
| opencode (外部) | — | 0 MB | ~50-100 MB/次 |
| zeroclaw (外部) | — | 0 MB | ~15-30 MB/次 |

# AI Agent Bridge (原 ai-chat 插件)

> **[DEPRECATED]** 
> 本项目原计划采用自研的基于 Rhai 脚本和 WASM 插件体系来实现 `ai-chat`。
> 经过重新评估资源的投入产出比和低内存环境（768MB VPS）的特性，决定**正式废弃**内置插件方案，转而采用 **External CLI Bridge (外部子进程桥接)** 的架构。
>
> 核心替代方案：**Opencode** 或 **Zeroclaw**。

## 1. 目标与范围

*   **目标**：在低资源环境 (768MB) 中提供极致强大的 AI 助手，同时**零维护成本**。
*   **范围**：Deve-Notebook 仅作为 UI 层（提供输入框流式对话展示、代码高亮），所有复杂的 Agent 逻辑（MCP 连接、本地环境读取、历史记录管理、流式推断上下文等）全权交由外部 CLI 工具处理。

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

## 3. 为什么选择成熟 CLI (Opencode / Zeroclaw)？

*   **历史管理**：它们内置了基于 sqlite 或本地 json 的历史状态机，支持 `/plan` 和 `/build` 等模式。
*   **工具支持 (Tools & MCP)**：直接继承了它们庞大的内置工具链（读写文件、Bash 执行），无需我们在 Rhai 中重复造轮子去对接 `mcp_list_tools`。
*   **Skills (自定义技能)**：Opencode/Zeroclaw 原生支持 `.opencode/skills/` 类似目录加载预设 Prompts。
*   **Token 优化**：滑动窗口和上下文合并在这些成熟库中已经做到了极致优化。

## 4. 交互与 UI (保持不变)

UI 层的逻辑与原来的验收标准保持一致：
*   移动端折叠 Chat Sheet。
*   保持支持 Markdown 渲染和代码块横向滚动。
*   错误捕获：当子进程返回非 0 状态码时，前端展示重试态。

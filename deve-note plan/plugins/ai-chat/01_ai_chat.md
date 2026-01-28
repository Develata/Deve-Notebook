# ai-chat 插件规划

## 1. 目标与范围

*   **目标**：在低资源环境 (768MB) 中提供可控、可扩展、低 Token 消耗的 AI 助手。
*   **范围**：插件仅负责对话与工具编排，模型与服务端能力通过 Host API 与 MCP 扩展。

## 2. 模式与交互

### Plan / Build
*   **Plan**：只输出计划，不调用工具、不修改代码。
*   **Build**：允许工具调用与代码修改。
*   **切换**：
    *   `/plan`、`/build` 直接切换。
    *   `/agents` 在 PLAN/BUILD 间顺序切换。

### 命令优先
*   主要入口通过命令面板，UI 只展示当前模式。

## 3. Token 优化策略

*   **滑动窗口**：保留最近 N 轮对话。
*   **工具压缩**：旧工具输出替换为简短占位。
*   **字符上限**：总字符数超过阈值时裁剪旧消息。

## 4. System Prompt 结构

*   **XML 结构**：
    *   `<role>`: Math-Architect 人设 + 中文输出。
    *   `<constraints>`: 130 行限制、768MB 约束、错误处理规则。
    *   `<context>`: 项目树、当前文件、Git 状态、Skills、MCP。
*   **目标**：降低幻觉，提升指令服从。

## 5. Tools 与 MCP

*   **内置工具**：读写文件、Git 状态、diff、commit。
*   **MCP 动态工具**：通过 `mcp_list_tools` 动态拼接为 `mcp::server::tool`。
*   **MCP 传输**：
    *   HTTP JSON-RPC
    *   SSE JSON-RPC
    *   Stdio JSON-RPC

## 6. Skills 机制

*   **发现路径**：
    *   `.deve/skills`
    *   `.opencode/skill` / `.opencode/skills`
    *   `.claude/skills`
*   **调用方式**：`/skill <name>` 或 `use_skill` 工具。
*   **加载策略**：按需注入，避免默认膨胀。

## 7. 稳定性与安全

*   **权限检查**：所有 Host API 需 Capability 校验。
*   **迭代上限**：限制 tool call 循环次数，防止死循环。
*   **异常降级**：网络/工具异常时返回清晰错误字符串。

## 8. 测试与验收

*   **Rhai 加载**：插件脚本必须通过引擎复杂度限制。
*   **模式切换**：/plan /build /agents 逻辑可重复切换。
*   **MCP**：无配置时不影响启动，有配置时可列出工具。

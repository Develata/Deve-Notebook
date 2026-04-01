# 10_ai_agent.md - AI Agent 篇 (AI Agent)

> AI 能力是 Deve-Notebook 的**第一方原生产品层**，不再归入插件章节。
> 当前主线是**最小原生 AI Chat**；外部 CLI Agent 仅作为可选的 Trusted 模式预留。
> 若无法建立明确的安全边界，外部 CLI Agent **MUST NOT** 默认启用，且当前 release **MAY** 完全不提供该能力。

## 1. 目标与范围

*   **目标**：在 768 MB 约束下提供一个可用、可控、可解释的 AI 入口。
*   **当前主线（Current Track）**：
    - **Native AI Chat**：最小原生聊天能力，支持读取当前 Markdown/上下文并回答。
    - **Native Modes**：原生支持 `PLAN` / `BUILD` 两种聊天模式。
    - **Trusted External Agent Bridge**：仅作为高级可选模式预留，不视为当前发布主线。
*   **明确不在当前范围内（Out of Scope for Now）**：
    - 原生 MCP 集成
    - 原生 Skills 装载
    - 原生复杂 Agent 自治状态机（多代理协作、长链自主规划、无限工具循环）
    - 原生 Source Control 写入自动化

## 2. Native AI Chat（默认能力）

Native AI Chat 是当前**默认 shipped** 的 AI 形态，属于第一方内建能力。

### 功能边界

*   **MUST** 支持：
    - 单轮/多轮 chat
    - 读取当前打开的 Markdown 文档
    - 读取必要的只读上下文（当前 repo、当前文件、用户显式附加的片段）
    - `/plan`：进入只读规划模式
    - `/build`：进入执行模式
    - `/agents`：在 `PLAN ↔ BUILD` 间顺序切换
*   **MUST NOT** 默认支持：
    - MCP
    - Skills
    - 直接 Source Control 写操作
    - 无用户确认的文件修改

### 设计原则

*   **Fail-closed**：拿不到上下文就降级为纯 chat，不得隐式扩大读取范围。
*   **Read-first**：当前阶段优先保证“读 markdown + chat”稳定，不追求工具丰富度。
*   **Low-memory**：常驻成本应保持在轻量级，适合低配环境。

### 原生模式定义

*   **PLAN 模式**：
    - **MUST NOT** 调用任何工具。
    - 只允许读取当前已提供的 Markdown / 上下文并产出计划、建议、分析。
    - 适合作为安全基线模式。
*   **BUILD 模式**：
    - **MAY** 直接修改当前 Markdown。
    - **MAY** 通过受控的程序执行路径完成 Markdown 修改或转换。
    - 上述执行路径 **MUST** 是受限的宿主能力，不得等价于开放式 shell / MCP / Skills。
    - 当前阶段默认只针对 Markdown 工作流，不自动扩展到任意工程文件。

## 3. Trusted External Agent Bridge（可选，高风险）

外部 CLI Agent 不再作为默认能力，而是**显式 opt-in 的 Trusted 模式**。

### 适用范围

*   **适用**：单用户、自托管、用户信任本机/本容器内 CLI 的场景。
*   **不适用**：多租户、公共托管、无法确认二进制来源的环境。

### 安全前提

若要原生支持 CLI Agent，至少必须满足以下条件：

1. **显式启用**：默认关闭，用户主动开启。
2. **可执行路径受控**：不得随意从 PATH 搜索任意命令；应有 allowlist 或固定路径。
3. **环境变量白名单**：子进程不得继承宿主全部环境。
4. **资源约束**：超时、输出上限、并发上限必须可控。
5. **读取边界清晰**：默认只给只读 Markdown / 明确导出的上下文，不直接暴露整个 live workspace。
6. **失败可退化**：安全壳不可用时，系统必须退回 Native AI Chat，而不是静默放开权限。

### 关键结论

*   如果上述边界在目标部署形态下无法成立，则 Deve-Notebook **SHOULD** 不内建 CLI Agent。
*   在这种情况下，用户可以自行在外部终端运行独立 CLI Agent；Deve-Notebook 不负责原生托管它。

## 4. 统一前端交互

前端仍共享同一套 Chat UI，但产品语义要清楚区分：

*   **Native AI Chat**：安全基线，默认模式。
*   **Trusted CLI Agent**：高级模式，明确标记为外部受信任代理。
*   **Native PLAN / BUILD**：属于 Native AI Chat 内部的交互模式，不等于外部 Agent Runtime。

统一要求：

*   Settings 中的模式切换必须清楚提示当前后端类型。
*   移动端 Chat Sheet、Markdown 渲染、错误重试逻辑共享。
*   `Disconnected`、`Unauthorized`、`Session Expired` 与 AI 请求失败必须分开处理。

## 5. 资源开销

| 能力 | 常驻内存 | 按需内存 | 当前定位 |
|------|---------|----------|----------|
| Native AI Chat | 轻量级 | SSE / provider response buffer | 默认 shipped |
| Trusted CLI Agent | 0 MB | 取决于外部 CLI | 可选，默认关闭 |

## 本章相关配置

*   `AI_API_KEY` / `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`: Native AI Chat 使用的服务密钥。
*   `AI_BASE_URL`: Native AI Chat API 端点。
*   `AI_MODEL`: Native AI Chat 模型名。
*   `AI_MAX_TOKENS`: Native AI Chat 最大 token 数。
*   `AGENT_CLI_PATH`: Trusted CLI Agent 可执行文件路径（仅在显式启用时读取）。

## 相关章节

*   外部 Agent / 计算运行时接口预留见 [17_plugins.md](./17_plugins.md)。
*   统一命令入口见 [12_commands.md](./12_commands.md)。
*   模式切换与安全开关见 [13_settings.md](./13_settings.md)。

# 16_ai_agent.md - AI Agent 篇 (AI Agent)

## Metadata

- `Layer`: `Peripheral / Optional Product Layer`
- `Status`: `Optional Product Layer`
- `Counterpart Feature`: `docs/features/10_ai_agent.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/10_plugins.md`
- `Primary Code Areas`: `apps/cli/src/server/ai_chat/`, `apps/cli/src/server/agent_bridge/`, `apps/web/src/components/chat/`, `apps/web/src/api/ai_backend.rs`, `crates/core/src/plugin/runtime/chat_stream.rs`
- `Related Design Notes`: [`docs/ai-chat-streaming.md`](../ai-chat-streaming.md) (streaming bridge design), [`docs/plan/plugins/agent_bridge/01_agent_bridge.md`](./plugins/agent_bridge/01_agent_bridge.md) (dual-channel architecture)

> AI 是可选第一方产品层。启用 AI 时，默认后端 **MUST** 是最小 Native AI Chat；Trusted CLI Agent 仅作显式 opt-in。MCP 不进入产品路线，代码层不得保留 MCP runtime / manager / host tool 入口。

## 1. Scope (目标与范围)

*   **目标**：在 768 MB 约束下提供一个可用、可控、可解释的可选 AI 入口。
*   **主线能力**：
    - **Native AI Chat**：启用 AI 功能时的最小原生聊天能力，支持读取当前 Markdown/上下文并回答。
    - **Native Modes**：原生支持 `PLAN` / `BUILD` 两种聊天模式。
    - **Trusted External Agent Bridge**：仅作为高级可选模式预留，不视为核心主线。
*   **明确不在当前范围内（Out of Scope for Now）**：
    - 原生 MCP 集成；该方向由 Skills / 受控 CLI 工具调用替代
    - 原生 Skills 装载
    - 原生复杂 Agent 自治状态机（多代理协作、长链自主规划、无限工具循环）
    - 原生 Source Control 写入自动化

### MCP Retired Direction

MCP 不进入 Native AI Chat 或 Trusted Agent roadmap。替代方向：

1. Native AI Chat 保持 read-first、最小可用。
2. 若需要外部能力，通过用户显式启用的 Trusted CLI path，或未来 Skills 调用受控 CLI 工具完成。
3. MCP 相关文字只作为历史决策保留；不得新增或保留 MCP runtime、MCP server 管理或 MCP tool loop。

## 2. Native AI Chat {#native-ai-chat-runtime}

Native AI Chat 是启用 AI 功能时的默认第一方 AI 形态，属于内建可选能力。

### 功能边界

*   启用 Native AI Chat 的 release **MUST** 支持：
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
*   **Read-first**：Baseline 优先保证“读 Markdown + chat”稳定。
*   **Low-memory**：常驻成本应保持在轻量级，适合低配环境。

### 运行时合同

*   Server runtime 负责 provider 选择、流式响应、错误收敛、资源上限与 trusted-policy enforcement。
*   Web chat UI 负责能力探测、模式展示、请求构造、streaming 展示与 retry/error 状态。
*   若保留 Rhai/plugin-host 兼容 bridge，该 bridge **MAY** 提供 transport-agnostic chat stream，但 **MUST NOT** 把 Native AI Chat 升格为通用插件主线。
*   Native AI Chat **MUST** 保持本节 read-first 边界：provider 可流式返回结果，但 **MUST NOT** 静默获得任意 source-control 或 workspace 写权限。
*   兼容 AI chat 插件 **MUST NOT** 授予广泛文件读取或默认工具执行；上下文只能由 chat request 显式传入。
*   public `PluginCall` surface **MUST** 限定为 `chat`；helper/config/tool 函数属于内部实现细节，被外部调用时必须 fail-closed。
*   产品后端名称是 `native` / `trusted-cli`；`ai-chat` / `agent-bridge` 这类兼容 plugin id 只是内部 routing detail，必须经过显式转换层。
*   同步 `PluginResponse` text 或 error **MUST** 完成对应 chat request；缺 API key、Trusted CLI 被禁用或 policy error **MUST NOT** 让 UI 无限 streaming/loading。
*   `ai.native_enabled = false` **MUST** 同时禁用 server provider registration 与 public `ai-chat` RPC；backend capabilities endpoint **MUST** 暴露该状态，让 Web UI 禁用或回退 Native backend。

### 原生模式定义

*   **PLAN 模式**：
    - **MUST NOT** 调用任何工具。
    - 只允许读取当前已提供的 Markdown / 上下文并产出计划、建议、分析。
    - 适合作为安全基线模式。
*   **BUILD 模式**：
    - **MAY** 直接修改当前 Markdown。
    - **MAY** 通过受控的程序执行路径完成 Markdown 修改或转换。
    - 上述执行路径 **MUST** 是受限的宿主能力，不得等价于开放式 shell / MCP / Skills。
    - Baseline 默认只针对 Markdown 工作流，不自动扩展到任意工程文件。
*   **`/agents` 模式切换**：
    - 仅在原生 `PLAN ↔ BUILD` 间顺序切换。
    - **MUST NOT** 用作后端切换命令。
    - **MUST NOT** 隐式拉起 Trusted External Agent。

## 3. Trusted External Agent Bridge {#trusted-agent-bridge}

外部 CLI Agent 不再作为默认能力，而是**显式 opt-in 的 Trusted 模式**。

### 适用范围

*   **适用**：单用户、自托管、用户信任本机/本容器内 CLI 的场景。
*   **不适用**：多租户、公共托管、无法确认二进制来源的环境。

### 安全前提

若要原生支持 CLI Agent，至少必须满足以下条件：

1. **显式启用**：默认关闭，用户主动开启。
2. **可执行路径受控**：不得随意从 PATH 搜索任意命令；应有 allowlist 或固定路径，且路径必须存在并指向可执行文件。
3. **环境变量白名单**：子进程不得继承宿主全部环境。
4. **资源约束**：超时、输出上限、并发上限必须可控。
5. **读取边界清晰**：默认只提供只读 Markdown / 显式上下文，不暴露整个 live workspace。
6. **失败可退化**：安全壳不可用时，系统必须退回 Native AI Chat，而不是静默放开权限。

### 关键结论

*   如果上述边界无法成立，Deve-Notebook **SHOULD** 不内建 CLI Agent；用户可在外部终端自行运行。
*   若保留 `agent-bridge`，它只能是 default-off、policy-gated 的 Trusted CLI path；它不得被描述成通用插件市场能力，也不得绕过 `AGENT_CLI_PATH` / trusted-mode gating。
*   `AGENT_CLI_PATH` 必须解析为显式绝对路径；子进程必须清空默认环境、设置超时、输出上限和并发上限，避免退化成开放式 shell/agent runner。
*   `DEVE_AI_AGENT_BRIDGE_ENABLED` 与 `DEVE_AI_AGENT_BRIDGE_TRUSTED` 是
    `ai.agent_bridge.enabled` / `ai.agent_bridge.trusted` 的兼容环境变量别名；它们只改变
    Trusted CLI policy 输入，不授予额外能力，也不得绕过 `AGENT_CLI_PATH` 的绝对路径与可执行文件检查。

## 4. Unified Frontend Interaction (统一前端交互)

前端仍共享同一套 Chat UI，但产品语义要清楚区分：

*   **Native AI Chat**：启用 AI 功能时的安全基线与默认模式。
*   **Trusted CLI Agent**：高级模式，明确标记为外部受信任代理。
*   **Native PLAN / BUILD**：属于 Native AI Chat 内部的交互模式，不等于外部 Agent Runtime。

统一要求：

*   Settings 中的模式切换必须清楚提示当前后端类型。
*   Native `PLAN / BUILD` 与 `Backend(native / trusted-cli)` 必须是两组独立概念，不得混用。
*   移动端 Chat Sheet、Markdown 渲染、错误重试逻辑共享。
*   `Disconnected`、`Unauthorized`、`Session Expired` 与 AI 请求失败必须分开处理。

## 5. Resource Budget (资源开销)

| 能力 | 常驻内存 | 按需内存 | 定位 |
|------|---------|----------|----------|
| Native AI Chat | 轻量级 | SSE / provider response buffer | 启用 AI 功能时的第一方基线 |
| Trusted CLI Agent | 0 MB | 取决于外部 CLI | 可选，默认关闭 |

## 6. Related Configuration (本章相关配置)

*   `AI_API_KEY` / `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`: Native AI Chat 使用的服务密钥。
*   `AI_BASE_URL`: Native AI Chat API 端点。
*   `AI_MODEL`: Native AI Chat 模型名。
*   `AI_MAX_TOKENS`: Native AI Chat 最大 token 数。
*   `AGENT_CLI_PATH`: Trusted CLI Agent 可执行文件路径（仅在显式启用时读取）。
*   `DEVE_AI_AGENT_BRIDGE_ENABLED`: `ai.agent_bridge.enabled` 的兼容环境变量别名。
*   `DEVE_AI_AGENT_BRIDGE_TRUSTED`: `ai.agent_bridge.trusted` 的兼容环境变量别名。

## 7. Related Chapters (相关章节)

*   外部 Agent / 计算运行时接口预留见 [19_plugins.md](./19_plugins.md)。
*   统一命令入口见 [14_commands.md](./14_commands.md)。
*   模式切换与安全开关见 [15_settings.md](./15_settings.md)。

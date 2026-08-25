# 16_ai_agent.md - AI Agent 篇 (AI Agent)

## Metadata

- `Layer`: `Peripheral / Optional Product Layer`
- `Status`: `Optional Product Layer`
- `Version`: `0.0.1`
- `Last Review`: `2026-08-25`
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
*   第一方 Native AI runtime 的 manifest 与受控 Rhai script 必须作为编译期资源嵌入 server/native
    binary；外部 `plugins/` 目录只承载可选插件，Android/Desktop bundle 不得依赖运行目录恰好存在
    `plugins/ai-chat`。
*   内建 runtime 与外部 runtime 必须进入同一注册表；相同 plugin id 出现两次时启动 fail-closed，
    不允许以加载顺序静默覆盖。嵌入资源不得包含 API key、cookie、session 或其他 secret。
*   backend capabilities 的 `native_available` 必须由有效配置与当前 runtime 注册结果共同决定；仅有
    `ai.native_enabled = true` 但内建注册失败时必须报告不可用，不能让 UI 进入必然失败的发送路径。
*   Provider secret 与 provider selection 由 server-owned `NativeAiProviderSettingsRuntime` 解析并在请求
    admission 时取得 immutable snapshot；Rhai bridge 只传 `req_id + history + tools(None)`，不得读取环境、
    拼 provider URL、携带 API key 或拥有 network authority。
*   Chat stream handler 是同一 server binary 的进程级基础设施：同一内建实现的重复初始化必须幂等，以支持
    Android 同进程 embedded backend generation replacement；不同实现或外部替换仍必须 fail-closed。
*   Web Chat 的每条本地消息 **MUST** 在集中构造点取得稳定的 UI-only identity；该 identity **MUST NOT**
    由 `req_id`、可变消息正文、role 或时间戳派生，也不得序列化为 server/protocol 字段。相同请求前缀、
    相同毫秒时间或无 `req_id` 的消息仍必须保持独立 identity。
*   Chat keyed row **MUST** 以该 identity 作为 key，并按 identity 响应式读取当前消息正文；流式 delta
    到达时必须更新同一 row/DOM，而不得因可变正文变化而重建或复用错误消息。Markdown、TeX 与代码高亮
    仅是消息 body 的 presentation projection；单个 delta 的投影范围应限定在发生变化的消息 body，
    不得重新渲染或重新高亮整个聊天历史。消息结构索引的性能预算与增量维护策略属于
    `21_perf_budget#critical-path-budget`，不得用改变消息 identity 或正文权威来换取性能。
*   只有编译期内建 `ai-chat` runtime 可以注册 chat stream host API 并取得 request-local stream scope。
    外部 Rhai plugin 即使猜中 host function 名称也必须得到 unknown-function/capability denial，不能借用 server
    provider secret、网络出口或付费调用额度。
*   三种 provider adapter 是同一 runtime boundary 下的 peer implementations：
    `openai-chat-completions`、`openai-responses`、`anthropic-messages`。每个 adapter 独占 endpoint、认证、
    request projection 和 SSE parser；不得用容错 JSON 猜测把一种协议伪装成另一种。
*   OpenAI Responses 只接受 output text delta；function/tool call、refusal 或 response failed 必须结束请求并
    fail-closed。Anthropic Messages 只接受 text content block；`tool_use`/`server_tool_use`、input JSON delta、
    thinking 与 error event 不得投影为可执行工具或普通回答。未知非关键事件可以忽略，未知内容块必须
    fail-closed。
*   OpenAI Chat Completions 只接受 `assistant` 文本 delta 与正常 `stop`；refusal、content filter、tool/function
    signal、未知 delta 字段或未知 finish reason 必须 fail-closed，不能把不完整/被拒绝结果伪装为成功文本。
*   Provider stream 必须观察到所属协议的合法终态（Chat `stop`、包含 `status=completed` 与可验证 text-only
    `output` 的 Responses `response.completed`、Anthropic
    `message_delta.stop_reason`）后才能投影成功；`[DONE]`、EOF 或 transport `StreamEnded` 仅是 framing/连接终止，
    不得自行制造成功终态。

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
*   Chat message Markdown rendering **MAY** include TeX display via the auxiliary KaTeX projection defined in `10_rendering`; this is presentation-only and **MUST NOT** expand context reads, tool calls, source-control writes, or workspace write permission.
*   `Disconnected`、`Unauthorized`、`Session Expired` 与 AI 请求失败必须分开处理。

## 5. Resource Budget (资源开销)

| 能力 | 常驻内存 | 按需内存 | 定位 |
|------|---------|----------|----------|
| Native AI Chat | 轻量级 | SSE / provider response buffer | 启用 AI 功能时的第一方基线 |
| Trusted CLI Agent | 0 MB | 取决于外部 CLI | 可选，默认关闭 |

## 6. Related Configuration (本章相关配置)

*   `AI_API_KEY` / `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`: Native AI Chat 使用的服务密钥。
*   `AI_PROVIDER`: `openai-chat-completions` / `openai-responses` / `anthropic-messages`。
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

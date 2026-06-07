# 10_ai_agent.md - AI 功能体验篇

本章描述当前阶段用户在产品里实际能看到的 AI 相关能力与边界。

## 功能目标

- AI 入口不能破坏 Markdown、Repo、Source Control 等核心工作流。
- 用户应明确知道哪些是当前可用能力，哪些只是外围或未来扩展。

## Operation 示例

- Native AI Chat 原子操作示例见 `docs/features/operations/ai_chat.md`。
- 该示例将 AI chat 拆为打开面板、输入 prompt、发送消息、接收流式结果四个 user operations。
- Trusted External Agent 边界示例见 `docs/features/operations/trusted_external_agent_boundary.md`。
- 该示例把 `trusted-cli` 拆成 settings 入口、后端切换、提交、disabled fallback、流式返回五个 user operations。

## 功能项

### 1. 当前 AI 入口

- 用户可以看到统一的 AI 入口或聊天面板。
- AI 能力是外围辅助，不应抢占核心工作流。
- AI Chat 消息体支持 Markdown 展示，并可把 `$...$` / `$$...$$` 公式显示为 KaTeX 结果。

### 2. 当前暴露边界

- 当前阶段允许的 AI 能力应保持最小化与可理解。
- AI 不应假装拥有未完成的自动化写入、越权读取或复杂自治能力。
- Native AI 后端默认拒绝 tools payload 与 provider tool calls；当前 BUILD 只允许后续受控 Markdown Apply 路径。
- 后端产品语义使用 `native` / `trusted-cli`；`ai-chat` / `agent-bridge` 只是当前兼容层 runtime plugin id，不应出现在 Settings 持久化语义中。
- AI 未配置、API key 缺失或 Trusted CLI policy fail-closed 时，必须显示明确文本/错误并结束 loading，不得卡住聊天面板。
- Chat 公式渲染只是展示层能力；不得扩大 AI 上下文读取范围、工具调用范围或 Markdown 写入权限。

### 3. 与核心功能的隔离

- AI 失败、不可用或未配置时，不应影响 Markdown 编辑、Repo 切换和 Source Control 基础功能。
- AI UI 只是额外入口，不是核心控制面的唯一方式。

## 非目标

- 当前阶段不把 AI 视为核心主线能力。
- 当前阶段不允许 AI 伪装成“已经完整接管编辑或 Source Control”。

## Chrome MCP 验收实例

### AI-FEAT-01: AI 入口不越权干扰核心工作流

前置条件：

- 打开应用主界面。

步骤：

1. 打开 AI 入口。
2. 观察它对编辑器、侧栏、Source Control 的影响。
3. 在 AI 面板关闭后继续正常编辑文档或切换仓库。

期望结果：

- AI 面板可以打开与关闭。
- 它不会阻断核心工作流，也不会伪装成核心控制入口。

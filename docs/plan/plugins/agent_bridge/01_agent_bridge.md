# AI 双通道边界 (Dual-Channel AI Boundary)

## Metadata

- `Layer`: `Peripheral / Design Note`
- `Status`: `Non-Authoritative Design Note`
- `Version`: `0.0.1`
- `Last Review`: `2026-05-24`
- `Counterpart Plan`: `docs/plan/16_ai_agent.md`, `docs/plan/19_plugins.md`
- `Primary Code Areas`: `apps/cli/src/server/agent_bridge/`, `apps/web/src/components/chat/`

> 本文只汇总 Native AI Chat 与 Trusted External Agent Bridge 边界；权威约束以 `docs/plan/16_ai_agent.md` 与 `docs/plan/19_plugins.md` 为准。Native AI Chat 是默认第一方能力；Trusted External Agent Bridge 是 default-off / trusted-only 高级部署位。

| 通道 | 定位 | 运行时开销目标 | 适用场景 |
|------|------|---------------|----------|
| **Native AI Chat** | 第一方聊天能力 | 常驻轻量，按需 streaming buffer | 用户自带 API Key 的简单问答 + 当前 Markdown / 显式上下文 |
| **Trusted Agent Bridge** | 外部 CLI 子进程桥接 | 常驻接近零，外部 CLI 按需启动 | 受信任单用户部署中的高级 Agent 能力 |

## 1. 目标与范围

*   **目标**：在低资源环境中提供可控 AI 助手，且默认不扩大工具权限。
*   **范围**：
    - **Native AI Chat**：Deve-Notebook 提供第一方聊天 UI 与 OpenAI-compatible streaming runtime。
    - **Trusted Agent Bridge**：只在 enabled + trusted + explicit executable path 条件满足时桥接受信任外部 CLI。

## 2. 外部 CLI 桥接边界

Trusted Agent Bridge 的边界摘要：

1. **显式启用**：默认关闭，由用户配置启用。
2. **受控可执行路径**：读取显式 `AGENT_CLI_PATH`，不退化为任意 PATH 搜索。
3. **环境隔离**：子进程只接收 allowlist 环境变量。
4. **资源上限**：具备超时、输出上限与并发上限。
5. **只读默认**：默认只获得当前 Markdown / 显式上下文；写入路径回到第 10 章定义的 BUILD / controlled apply 边界。
6. **失败回退**：trusted policy 不成立时回退 Native AI Chat，而不是静默放开外部 CLI。

## 3. 为什么保留 Trusted Agent Bridge 接口位？

*   外部 CLI 自行管理历史、Skills 与复杂 agent 状态机。
*   Deve-Notebook 只负责桥接、安全前提、streaming 与错误边界。
*   MCP 不作为产品运行时方向；相关需求由外部 CLI 或未来 Skills + 受控 CLI 承载。

## 4. Native AI Chat 默认方案

Native AI Chat 的默认边界：

*   单轮/多轮对话。
*   OpenAI-compatible streaming。
*   读取当前 Markdown / 显式选择上下文。
*   不实现 MCP runtime。
*   不默认启用 Skills / source-control 写入。
*   不承担复杂 Agent 自治状态机。

## 5. 统一前端交互

UI 层共享同一套 Chat surface，并区分：

*   `native` backend：第一方 Native AI Chat。
*   `trusted-cli` backend：显式启用的外部 CLI Agent。
*   `PLAN / BUILD`：Native AI Chat 的会话模式，不等于 backend。

错误捕获区分 external CLI 非零退出、provider 网络错误、policy denied、Unauthorized 与 Disconnected，不合并为同一 loading/retry 状态。

# 📑 Deve-Note Plan - Master Index

**版本**: 0.0.1
**核心理念**: Git-Flow P2P Architecture, Trinity Isolation, Remote Dashboard.
**当前状态**: Phase 3-4 (Implementation In Progress). 设计文档持续追认当前工作树。

本文档已模块化，请参阅以下子文档获取详细规划：

## 📚 目录 (Table of Contents)

### Phase 1: Context & Definitions
1.  **[Terminology & Definitions](./01_terminology.md)**: 核心术语 (Ledger, Snapshot, Peer) 与规范性用语.
2.  **[Project Positioning](./02_positioning.md)**: 项目定位、核心边界 (Core MUST).

### Phase 2: Architecture & Storage
3.  **[Rendering Engine](./03_rendering.md)**: 编辑器内核、LaTeX 公式与解析优先级.
4.  **[Data Storage](./04_storage.md)**: 三库隔离 (Trinity Isolation)、数据恢复与灾备.
5.  **[Network Architecture](./05_network.md)**: P2P 拓扑、Web 面板约束与同步协议.

### Phase 3: Version Control & Logic
6.  **[Repository & Branching](./06_repository.md)**: 仓库管理、严格分支策略与 Spectator Mode.
7.  **[Diff Logic](./07_diff_logic.md)**: 内部和解逻辑 (Reconciliation) 与合并流程.

### Phase 4: User Interface
8.  **[UI Design](./08_ui_design.md)**: **Cursor-Style** 5-Column Grid, Modal Search & Fixed Outline.
9.  **[Authentication](./09_auth.md)**: 12-Factor Auth, Argon2 + JWT & WebSocket Security.
10. **[Internationalization](./10_i18n.md)**: 多语言策略 (leptos_i18n) 与错误码规范.

### Phase 5: Extensions & Operations
11. **[Plugins & Runtime](./11_plugins.md)**: **Dual-Engine** (Rhai/WASM) & OCI Container (Podman).
11b. **[AI Integration](./11b_ai_integration.md)**: 双通道 AI 架构 (Agent Bridge + AI Chat)。
12. **[Commands Summary](./12_commands.md)**: CLI 与 Command Palette 指令汇总.
13. **[Settings Summary](./13_settings.md)**: 环境变量与配置文件汇总.
14. **[Technology Stack](./14_tech_stack.md)**: **Redb + CodeMirror 6**, Native/Mobile 差异化选型.
15. **[Release Strategy](./15_release.md)**: License (MIT), Release Channels & CI/CD Pipelines.
16. **[Web Thin Client & Ledger Confirmation](./16_web_thin_client_ledger.md)**: Web 薄客户端写入模型、Ack 确认链与 repo-scoped write readiness.

---

## 文档状态与职责边界

为避免“实现做到一半才发现文档分层混乱”，本目录按以下方式解释：

*   **Current MUST（当前硬约束）**：`01`、`02`、`04`、`05`、`06`、`07`、`09`、`10`。这些章节定义当前实现必须满足的不变量、协议约束与错误契约。
*   **Current UI Contract（当前界面契约）**：`03`、`08`。这些章节定义当前交互与可见行为，但不得改写 Ledger / Auth / Network 的权威规则。
*   **Approved Target Architecture（已批准目标架构）**：`16`。当 `04/05/07/09/10` 在 Web 写路径上存在交叉时，以 `16` 的收敛规则为准。
*   **Planned / Optional（规划或扩展）**：`11`、`11b`、`12`、`13`、`14`、`15`。这些章节可指导实现，但不得反向推翻 Current MUST。

### 章节归属规则

*   `04_storage.md`：Ledger、Projection、pending/staging、恢复与持久化边界。
*   `05_network.md`：连接拓扑、WS/HTTP 路由契约、repo-scoped sync handshake。
*   `09_auth.md`：user session、token 生命周期、鉴权失败处理。
*   `10_i18n.md`：错误码目录与前端文案映射，不负责传输层协议。
*   `16_web_thin_client_ledger.md`：Web thin-client 写入确认链、pending overlay、repo-scoped write readiness，以及 WS/HTTP 结构化错误契约在 Web 路径上的收敛。

### Global: Code Standards (代码规范)

*   **单文件行数限制**: 目标 < 130 行，MUST NOT 超过 250 行 (熔断阈值)。详见 `AGENTS.md` §2。
    *   **例外 — JS Bridge**: `apps/web/js/` 下的 JavaScript Bridge 文件因 FFI 性质，行数限制放宽至 target < 200 行, hard limit 400 行。
    *   **豁免 — 构建产物**: `*.bundle.js`、`dist/`、`target/` 等构建产物不受行数限制约束。此规则仅针对 **源文件 (Source Files)**。
*   **中文注释**: 每个模块/组件 SHOULD 包含中文文档注释。
*   **I18n 支持**: 所有用户可见文本 MUST 通过 `crate::i18n::t::xxx::yyy()` facade 获取，禁止直接把自然语言写进组件协议层。
*   **错误码**: 后端 MUST 返回结构化错误码 (e.g. `AUTH_INVALID_PASSWORD`)，前端据此映射到 `t::...` 文案；自然语言错误不得作为稳定协议契约。

### Appendix: Acceptance Test Cases
*   **[Acceptance Cases Index](./acceptance-cases/00_index.md)**: 验收用例集。

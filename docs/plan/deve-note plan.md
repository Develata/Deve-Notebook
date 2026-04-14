# 📑 Deve-Note Plan - Engineering Blueprint Index

**版本**: 0.0.1  
**定位**: `docs/plan/` 只保存工程蓝图、状态机、协议合同、模块边界与 refactor target。  
**配套文档**:

- [Feature Spec Index](../features/deve-note%20features.md)
- [Acceptance Case Index](../acceptance-cases/00_index.md)
- [Coverage Matrix](../coverage-matrix.md)
- [Gap & Audit Reports](../report/) — 阶段性差距分析与进度快照（时效性文档，非权威约束）

## 📚 Infra-First 目录

### A. Foundation
- **[00_engineering_constitution.md](./00_engineering_constitution.md)**: 工程宪法、骨架治理、四层调用图与对象平面的关系。
- **[01_terminology.md](./01_terminology.md)**: 核心术语、规范性用语、权威定义。
- **[02_positioning.md](./02_positioning.md)**: 产品边界、Core MUST / MUST NOT、项目定位。

### B. Authority Core
- **[04_storage.md](./04_storage.md)**: ledger、projection、workspace、watcher、repair 的存储蓝图。
- **[06_repository.md](./06_repository.md)**: repo identity、branch scope、tree projection、repo health。
- **[07_diff_logic.md](./07_diff_logic.md)**: pending/staging/commit/diff/merge 的 authority 路径。

### C. Runtime Protocols
- **[05_network.md](./05_network.md)**: P2P / WebLightPeer / relay / ws-http protocol / reconnect。
- **[09_auth.md](./09_auth.md)**: user session、入口鉴权、cookie/JWT、安全头与 TLS 合同。
- **[16_web_thin_client_ledger.md](./16_web_thin_client_ledger.md)**: Web 薄客户端 pending/ack/reject/write readiness。

### D. Application / UI Shell
- **[03_rendering.md](./03_rendering.md)**: editor runtime、parser、widget、outline、source-first 渲染蓝图。
- **[08_ui_design.md](./08_ui_design.md)**: shell/control/runtime 分层与多端共享控制接口。
- **[08_ui_design_01_web.md](./08_ui_design_01_web.md)**: Web shell adapter 与布局约束。
- **[08_ui_design_02_desktop.md](./08_ui_design_02_desktop.md)**: Desktop shell adapter 与原生边界。
- **[08_ui_design_03_mobile.md](./08_ui_design_03_mobile.md)**: Mobile shell adapter、gesture、drawer 约束。
- **[11_i18n.md](./11_i18n.md)**: i18n facade、错误码映射、文案约束。
- **[12_commands.md](./12_commands.md)**: command surface、palette、快捷键与 control 映射。
- **[13_settings.md](./13_settings.md)**: 设置、配置、持久化与 UI prefs 边界。

### E. Peripheral / Deferred
- **[10_ai_agent.md](./10_ai_agent.md)**: AI chat / trusted CLI / external runtime 的工程边界。
- **[14_tech_stack.md](./14_tech_stack.md)**: 技术栈与端侧 adapter 选型。
- **[15_release.md](./15_release.md)**: 构建、打包、发布、CI/CD。
- **[17_plugins.md](./17_plugins.md)**: plugin / external runtime 接口保留。

### F. Implementation Blueprints
- **[../tasks/18_infra_runtime.md](../tasks/18_infra_runtime.md)**: infra-first 模块拆分与运行时边界收敛蓝图。
- **[../tasks/19_repo_refactor_blueprint.md](../tasks/19_repo_refactor_blueprint.md)**: 仓库重构迁移顺序与目录调整蓝图。

---

## 文档状态与职责边界

为避免“实现做到一半才发现文档分层混乱”，本目录按以下方式解释：

*   **Current MUST（当前硬约束）**：`01`、`02`、`04`、`05`、`06`、`07`、`09`、`11`。这些章节定义当前实现必须满足的不变量、协议约束与错误契约。
*   **Current UI Contract（当前界面契约）**：`03`、`08`。这些章节定义当前交互与可见行为的工程实现，但不得改写 Ledger / Auth / Network 的权威规则。
*   **Approved Runtime Architecture（已批准运行时架构）**：`16`，以及 `04/06/07` 中的 Node/Path Ledger Facts 收敛路线。当 `04/05/07/09/11` 在 Web 写路径上存在交叉时，以 `16` 的 Web 收敛规则为准；当路径、树结构与 Source Control commit 存在交叉时，以 `04/06/07` 的 Node-first 约束为准。
*   **Planned / Optional（规划或扩展）**：`10`、`12`、`13`、`14`、`15`、`17`。这些章节可指导实现，但不得反向推翻 Current MUST。

### 文档分层

*   `docs/plan/`：工程蓝图，回答 how it is engineered。`00_engineering_constitution.md` 是跨章节骨架治理规则；普通章节不得与其冲突。
*   `docs/features/`：功能说明，回答 what the product does。
*   `docs/acceptance-cases/`：自动化验收，回答 automation proves it。
*   `docs/coverage-matrix.md`：`plan / features / acceptance-cases` 的章节映射表。

### Infra-First 阅读顺序

当任务属于核心功能（Markdown / P2P / Source Control / Repo Scope / Pending Writes / Recovery）时，推荐按以下顺序阅读：

1. `01_terminology.md` → 确认术语与规范性用语。
2. `00_engineering_constitution.md` → 确认骨架治理、层级模型与变更审批规则。
3. `02_positioning.md` → 确认当前阶段的 Core MUST / MUST NOT。
4. `04_storage.md`、`06_repository.md`、`07_diff_logic.md` → 确认 authority、projection、repo health、source control 的硬约束。
5. `05_network.md`、`09_auth.md`、`16_web_thin_client_ledger.md` → 确认协议、session、scope、pending write 的状态机。
6. `03_rendering.md`、`08_ui_design*.md`、`11_i18n.md`、`12_commands.md`、`13_settings.md` → 在不改写权威规则的前提下定义 shell / control / rendering 行为。
7. `docs/features/` 中对应章节 → 查看用户可见行为与 Chrome MCP 手工验收实例。
8. `docs/acceptance-cases/` 中对应章节 → 查看自动化验证入口。

### 章节归属规则

*   `04_storage.md`：Ledger、Projection、pending/staging、恢复与持久化边界。
    *   补充约束：`Content Facts + Structure Facts -> Projection`，`metadata/path/tree` 仅为投影结果，不是业务写入真值源。
*   `05_network.md`：连接拓扑、WS/HTTP 路由契约、repo-scoped sync handshake。
*   `06_repository.md`：`NodeId`、树结构、`Rename/Move/Create/Delete` 的结构事实写路径。
*   `07_diff_logic.md`：外部文件系统变更在 Stage -> Commit 时如何拆成内容事实与结构事实。
*   `09_auth.md`：user session、token 生命周期、鉴权失败处理。
*   `10_ai_agent.md`：原生 AI Chat 的产品边界、Trusted CLI Agent 的启用条件与 fail-closed 安全前提。
*   `11_i18n.md`：错误码目录与前端文案映射，不负责传输层协议。
*   `16_web_thin_client_ledger.md`：Web thin-client 写入确认链、pending overlay、repo-scoped write readiness，以及 WS/HTTP 结构化错误契约在 Web 路径上的收敛。

### 平台子章规则

*   `08_ui_design.md` 是共享 shell/control/runtime 总章。
*   `08_ui_design_01/02/03` 只描述 Web / Desktop / Mobile 的 adapter 与 surface 差异。
*   共享控制语义、authority 约束、runtime 归属不得在子章里重新定义或偏离总章。

### Route 2 Guardrail（Node/Path 一等事实护栏）

*   当实现进入 Node-first 重构时，路径与树结构相关的最终业务事实 **MUST** 进入 Ledger，而不是通过 `metadata` / `path cache` 直写完成。
*   `metadata`、`DocId <-> Path` 映射、`TreeDelta`、侧边栏树与 Vault 工作区都 **MUST** 视为 projection 或 projection cache。
*   `RenameDoc / MoveDoc / DeleteDoc / CreateDoc` 与 Source Control rename commit **MUST** 共享同一条结构事实写路径。

### Infra-First Guardrail（基础设施优先护栏）

*   控件、页面与视图层 **MUST NOT** 直接持有业务真相；它们只能消费 runtime 暴露的状态，并发出 typed intents。
*   feature runtime **MUST** 只拥有本功能的状态机，不得直接篡改其他 runtime 的内部状态。
*   authority core（ledger / facts / append validation）与 projection / repair **MUST** 明确分层；projection 失败不得回写 authority，也不得伪装为成功写入。
*   修复逻辑（repair / degraded / quarantine / stale scope recovery）**MUST** 视为第一类基础设施能力，而不是散落在 handler / effect / component 内的临时补丁。
*   plugin / AI / external runtime **MUST** 视为外围系统；在核心功能未稳定前，不得要求它们反向主导 authority、scope 或 pending write 的主链设计。

### Global: Code Standards (代码规范)

*   **单文件行数限制**: 目标 < 130 行，MUST NOT 超过 250 行 (熔断阈值)。详见 `AGENTS.md` §2。
    *   **例外 — JS Bridge**: `apps/web/js/` 下的 JavaScript Bridge 文件因 FFI 性质，行数限制放宽至 target < 200 行, hard limit 400 行。
    *   **豁免 — 构建产物**: `*.bundle.js`、`dist/`、`target/` 等构建产物不受行数限制约束。此规则仅针对 **源文件 (Source Files)**。
*   **中文注释**: 每个模块/组件 SHOULD 包含中文文档注释。
*   **I18n 支持**: 所有用户可见文本 MUST 通过 `crate::i18n::t::xxx::yyy()` facade 获取，禁止直接把自然语言写进组件协议层。
*   **错误码**: 后端 MUST 返回结构化错误码 (e.g. `AUTH_INVALID_PASSWORD`)，前端据此映射到 `t::...` 文案；自然语言错误不得作为稳定协议契约。

### Appendix
*   **[Feature Spec Index](../features/deve-note%20features.md)**: 功能说明与 Chrome MCP 手工验收实例。
*   **[Acceptance Cases Index](../acceptance-cases/00_index.md)**: 自动化验收用例集。

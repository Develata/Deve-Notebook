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
- **[03_storage/](./03_storage/index.md)**: ledger、projection、workspace、watcher、repair 的存储蓝图。
- **[04_repository.md](./04_repository.md)**: repo identity、branch scope、tree projection、repo health。
- **[05_diff_logic.md](./05_diff_logic.md)**: pending/staging/commit/diff/merge 的 authority 路径。
- **[06_backup.md](./06_backup.md)**: repo/branch URL 的备份展开、加密 pack、WebDAV/S3 边界。
- **[12_source_control_ui.md](./12_source_control_ui.md)**: Source Control view 的 VS Code-like UI contract。

### C. Runtime Protocols
- **[07_network.md](./07_network.md)**: P2P / WebLightPeer / relay / ws-http protocol / reconnect。
- **[08_auth.md](./08_auth.md)**: user session、入口鉴权、cookie/JWT、安全头与 TLS 合同。
- **[09_web_thin_client_ledger.md](./09_web_thin_client_ledger.md)**: Web 薄客户端 pending/ack/reject/write readiness。

### D. Application / UI Shell
- **[10_rendering.md](./10_rendering.md)**: editor runtime、parser、widget、outline、source-first 渲染蓝图。
- **[11_ui_design/](./11_ui_design/index.md)**: shell/control/runtime 分层与多端共享控制接口。
- **[11_ui_design/01_web.md](./11_ui_design/01_web.md)**: Web shell adapter 与布局约束。
- **[11_ui_design/02_desktop.md](./11_ui_design/02_desktop.md)**: Desktop shell adapter 与原生边界。
- **[11_ui_design/03_mobile.md](./11_ui_design/03_mobile.md)**: Mobile shell adapter、gesture、drawer 约束。
- **[13_i18n.md](./13_i18n.md)**: i18n facade、错误码映射、文案约束。
- **[14_commands.md](./14_commands.md)**: command surface、palette、快捷键与 control 映射。
- **[15_settings.md](./15_settings.md)**: 设置、配置、持久化与 UI prefs 边界。

### E. Peripheral / Deferred
- **[16_ai_agent.md](./16_ai_agent.md)**: AI chat / trusted CLI / external runtime 的工程边界。
- **[17_tech_stack.md](./17_tech_stack.md)**: 技术栈与端侧 adapter 选型。
- **[18_release.md](./18_release.md)**: 构建、打包、发布、CI/CD。
- **[19_plugins.md](./19_plugins.md)**: plugin / external runtime 接口保留。

### F. Implementation Blueprints
- **[../tasks/18_infra_runtime.md](../tasks/18_infra_runtime.md)**: infra-first 模块拆分与运行时边界收敛蓝图。
- **[../tasks/19_repo_refactor_blueprint.md](../tasks/19_repo_refactor_blueprint.md)**: 仓库重构迁移顺序与目录调整蓝图。
- **[../tasks/20_web_thin_client_ledger_migration.md](../tasks/20_web_thin_client_ledger_migration.md)**: Web thin-client ledger 写入链路迁移顺序。

---

## 文档状态与职责边界

本目录按以下状态解释：

*   **Current MUST（当前硬约束）**：`01`、`02`、`04`、`05`、`06`、`07`、`09`、`11`。定义必须满足的不变量、协议约束与错误契约。
*   **Current UI Contract（当前界面契约）**：`03`、`08`。定义交互与可见行为，但不得改写 Ledger / Auth / Network 权威规则。
*   **Approved Runtime Architecture（已批准运行时架构）**：`16`，以及 `04/06/07` 中的 Node/Path Ledger Facts 收敛路线。当 `04/05/07/09/11` 在 Web 写路径上存在交叉时，以 `16` 的 Web 收敛规则为准；当路径、树结构与 Source Control commit 存在交叉时，以 `04/06/07` 的 Node-first 约束为准。
*   **Optional Product Layer（可选产品层）**：`10` 定义 Native AI Chat 的启用后合同，以及 Trusted CLI Agent 的显式 opt-in 边界；它不得反向推翻 Current MUST，也不得成为核心数据路径的隐式依赖。
*   **Planned / Optional（规划或扩展）**：`12`、`13`、`14`、`15`、`17`、`18`。可指导实现，但不得推翻 Current MUST。
*   **Current View Contract（当前视图契约）**：`19` 定义 Source Control view 的 VS Code-like 信息架构与交互模型；它不得推翻 `07` 的 Source Control authority。

### 文档分层

*   `docs/plan/`：工程蓝图，回答 how it is engineered；`00_engineering_constitution.md` 是跨章节治理规则。
*   `docs/features/`：功能说明，回答 what the product does。
*   `docs/acceptance-cases/`：自动化验收，回答 automation proves it。
*   `docs/coverage-matrix.md`：`plan / features / acceptance-cases` 的章节映射表。

### Infra-First 阅读顺序

当任务属于核心功能（Markdown / P2P / Source Control / Repo Scope / Pending Writes / Recovery）时，推荐按以下顺序阅读：

1. `01_terminology.md` → 确认术语与规范性用语。
2. `00_engineering_constitution.md` → 确认骨架治理、层级模型与变更审批规则。
3. `02_positioning.md` → 确认当前阶段的 Core MUST / MUST NOT。
4. `03_storage/`、`04_repository.md`、`05_diff_logic.md` → 确认 authority、projection、repo health、source control 的硬约束。
5. `07_network.md`、`08_auth.md`、`09_web_thin_client_ledger.md` → 确认协议、session、scope、pending write 的状态机。
6. `10_rendering.md`、`11_ui_design/*`、`12_source_control_ui.md`、`13_i18n.md`、`14_commands.md`、`15_settings.md` → 在不改写权威规则的前提下定义 shell / control / rendering 行为。
7. `docs/features/` 中对应章节 → 查看用户可见行为与 Chrome MCP 手工验收实例。
8. `docs/acceptance-cases/` 中对应章节 → 查看自动化验证入口。

### 章节归属规则

*   `03_storage/`：Ledger、Projection、pending/staging、恢复与持久化边界。
    *   补充约束：`Content Facts + Structure Facts -> Projection`，`metadata/path/tree` 仅为投影结果，不是业务写入真值源。
*   `07_network.md`：连接拓扑、WS/HTTP 路由契约、repo-scoped sync handshake。
*   `04_repository.md`：`NodeId`、树结构、`Rename/Move/Create/Delete` 的结构事实写路径。
*   `05_diff_logic.md`：外部文件系统变更在 Stage -> Commit 时如何拆成内容事实与结构事实。
*   `06_backup.md`：repo/branch URL 如何扩展为 WebDAV/S3 backup locator；备份不得成为共享可写 sync authority。
*   `12_source_control_ui.md`：Source Control view 如何参考 VS Code SCM mental model；不得复制 VS Code implementation 或改写 Source Control authority。
*   `08_auth.md`：user session、token 生命周期、鉴权失败处理。
*   `16_ai_agent.md`：原生 AI Chat 的产品边界、Trusted CLI Agent 的启用条件与 fail-closed 安全前提。
*   `13_i18n.md`：错误码目录与前端文案映射，不负责传输层协议。
*   `09_web_thin_client_ledger.md`：Web thin-client 写入确认链、pending overlay、repo-scoped write readiness，以及 WS/HTTP 结构化错误契约在 Web 路径上的收敛。

### 平台子章规则

*   `11_ui_design/` 是共享 shell/control/runtime 总章。
*   `08_ui_design_01/02/03` 只描述 Web / Desktop / Mobile 的 adapter 与 surface 差异。
*   共享控制语义、authority 约束、runtime 归属不得在子章里重新定义或偏离总章。

### Route 2 Guardrail（Node/Path 一等事实护栏）

*   当实现进入 Node-first 重构时，路径与树结构相关的最终业务事实 **MUST** 进入 Ledger，而不是通过 `metadata` / `path cache` 直写完成。
*   `metadata`、`DocId <-> Path` 映射、`TreeDelta`、侧边栏树与 Projection Workspace 都 **MUST** 视为 projection 或 projection cache。
*   `RenameDoc / MoveDoc / DeleteDoc / CreateDoc` 与 Source Control rename commit **MUST** 共享同一条结构事实写路径。

### Infra-First Guardrail（基础设施优先护栏）

*   控件、页面与视图层 **MUST NOT** 直接持有业务真相；它们只能消费 runtime 暴露的状态，并发出 typed intents。
*   feature runtime **MUST** 只拥有本功能的状态机，不得直接篡改其他 runtime 的内部状态。
*   authority core（ledger / facts / append validation）与 projection / repair **MUST** 明确分层；projection 失败不得回写 authority，也不得伪装为成功写入。
*   修复逻辑（repair / degraded / quarantine / stale scope recovery）**MUST** 视为第一类基础设施能力，而不是散落在 handler / effect / component 内的临时补丁。
*   plugin / AI / external runtime **MUST** 视为外围系统；在核心功能未稳定前，不得要求它们反向主导 authority、scope 或 pending write 的主链设计。

### Runtime Skeleton Registry

Runtime 名称、收敛状态、当前代码承载路径与 tracking task 统一维护在
[`docs/registry/runtime-skeleton-registry.md`](../registry/runtime-skeleton-registry.md)。

本章不再复制状态表，避免把 `docs/plan/` 变成随实现频繁变化的进度文档。各章
`Refactor Target` 与 registry 冲突时，先核对当前代码；若 registry 过时，更新
registry；若 plan 边界本身需要改变，必须按 `00_engineering_constitution.md`
的骨架治理规则处理。

### Global: Code Standards (代码规范)

*   **单文件内聚检查**: 按职责/API/基础设施边界拆分，不按行数机械拆分；超过 250 行为软架构警告，超过 500 行为熔断阈值。测试、test support 与 `apps/web/js/` bridge 可因上下文内聚超过软阈值；构建产物不受此规则约束。
*   **中文注释**: 每个模块/组件 SHOULD 包含中文文档注释。
*   **I18n 支持**: 所有用户可见文本 MUST 通过 `crate::i18n::t::xxx::yyy()` facade 获取，禁止直接把自然语言写进组件协议层。
*   **错误码**: 后端 MUST 返回结构化错误码 (e.g. `AUTH_INVALID_PASSWORD`)，前端据此映射到 `t::...` 文案；自然语言错误不得作为稳定协议契约。

### Appendix
*   **[Feature Spec Index](../features/deve-note%20features.md)**: 功能说明与 Chrome MCP 手工验收实例。
*   **[Acceptance Cases Index](../acceptance-cases/00_index.md)**: 自动化验收用例集。

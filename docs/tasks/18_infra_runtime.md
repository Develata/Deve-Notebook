# 18_infra_runtime.md - 基础设施与运行时边界篇 (Infra & Runtime Boundary)

## 1. 章节状态

本章为 **Approved Runtime Architecture**。它不取代 `04/05/06/07/09/16` 的权威约束，而是把这些章节翻译为 infra-first 的实现边界。

本章适用于：

* 核心功能模块拆分
* 运行时状态机定义
* 故障恢复与 repair 设计
* repo 级 / doc 级 / sync 级边界收敛

本章 **MUST NOT** 被用于放宽 Ledger-first、UUID-first、fail-closed 与 repo-scoped 协议要求。

## 2. Infra-First 基本原则

* **Single Responsibility by Module**：每个模块只负责一种职责，不允许“顺手”接管别的业务模块的内部状态。
* **Authority Before Projection**：权威事实先于投影；投影失败不得污染 authority。
* **Typed Collaboration**：跨模块协作必须通过 typed command / event / query 完成，禁止隐式共享状态与名字猜测。
* **Repair Is First-Class**：repair / degraded / quarantine / stale-scope recovery 是正式基础设施能力，不是临时补丁。
* **UI Is Not Truth**：组件与页面只负责展示与意图发射，不负责保存业务真相。

## 3. 运行时分层 (Runtime Layering)

### 3.1 Authority Core

职责：

* ledger append / query
* content facts / structure facts
* append validation
* repo identity / doc identity / node identity
* 去重、索引与权威序列号

规则：

* authority core 是唯一真值源。
* 任何结构写入必须在 append 前完成合法性校验。
* authority 层不得依赖 UI 形态、页面状态或组件语义。

### 3.2 Projection & Repair

职责：

* tree projection
* workspace projection
* materialize / rebuild
* degraded / quarantine 状态管理
* 历史坏账本的受控 repair

规则：

* projection 是派生视图，不是权威写源。
* projection 失败必须显式进入 degraded / quarantined 路径。
* repair 必须有清晰入口与日志，不得散落在任意 handler / effect 内偷偷执行。

### 3.3 Scope & Session Runtime

职责：

* user session
* websocket session
* repo scope
* branch scope
* doc scope
* scope nonce / stale scope cleanup

规则：

* user auth 与 peer/sync identity 必须分层。
* repo scope 必须按 `repo_id` 隔离，不允许通过名称猜测跨 repo 复用状态。
* stale scope 命中坏 repo 时，必须走显式恢复流程，而不是静默绑定到“看起来最像”的 repo。

### 3.4 Document Runtime

职责：

* OpenDoc / Snapshot / History / NewOp
* pending local edits
* ack / reject
* navigation guard
* 编辑器局部投影（outline、selection、render hints）

规则：

* document runtime 必须拥有独立状态机。
* “已写入 ledger 但后续持久化失败” 与 “明确 reject” 必须是不同状态。
* UI 不得把“继续离开视图”误导成“写入已提交”。

### 3.5 Source Control Runtime

职责：

* staged / unstaged
* history / graph
* diff session
* commit pipeline
* local vs remote readonly 边界

规则：

* Source Control 不得直接拥有 repo scope 真相，只能消费 scope runtime 暴露的当前作用域。
* remote / spectator 只读约束必须由 runtime 与 server 双边 enforce。

### 3.6 UI Shell & Feature Views

职责：

* 组件布局
* 视觉层级
* 输入交互
* typed intent 发射

规则：

* 组件不得直接持有 authority 状态。
* 组件不得绕过 runtime 直接操作业务写路径。
* “页面切换 / 菜单展开 / drawer 状态” 属于 UI shell，本地即可；repo/doc/pending/sync 不属于 UI shell。

### 3.7 Peripheral Systems

职责：

* AI
* plugins
* external agent bridges
* calculation runtimes

规则：

* 外围系统不得主导 authority / scope / pending write 主链。
* 在核心功能未稳定前，它们必须服从核心运行时边界。

## 4. 跨层协作合同 (Cross-Layer Contracts)

### 4.1 UI -> Runtime

UI 只能发出 typed intents，例如：

* `OpenDoc(doc_id)`
* `SwitchRepo(repo_id)`
* `StageFile(doc_id)`
* `CommitChanges(message)`

禁止：

* UI 直接写 ledger
* UI 直接修 projection cache
* UI 直接修改其他 runtime 的内部字段

### 4.2 Runtime -> Authority

runtime 只能通过明确命令调用 authority：

* append content facts
* append structure facts
* query authoritative state

禁止：

* 先写 projection 再倒推 authority
* 用 path cache / metadata 伪造权威状态

### 4.3 Authority -> Projection

projection 必须消费 authority 结果或 authoritative append 事件：

* tree rebuild
* workspace materialize
* repo-scoped cache refresh

禁止：

* handler 直接篡改 tree/path cache 然后宣称写入成功

## 5. 故障与恢复模型 (Failure & Recovery Model)

### 5.1 Repo Health States

每个 repo 至少应具有以下健康态：

* `Healthy`
* `Degraded`
* `Repairing`
* `Quarantined`

### 5.2 Projection Failure

* projection 失败时，repo **MUST** 进入显式 degraded 或 quarantined。
* degraded repo **MAY** 提供受限 fallback（例如 docs-only），但必须保持只读或受控写入边界。
* fallback 不是长期目标，repair 才是正式路径。

### 5.3 Pending Write Failure

pending write 至少必须区分：

* `WaitingForAck`
* `Committed`
* `Rejected`
* `CommittedButWritebackFailed`

禁止把以上状态全部折叠成“还在等 ack”。

### 5.4 Scope Failure

当 persisted scope 命中坏 repo 或失效 branch 时：

* 必须显式清理 stale scope
* 必须重新请求健康 repo 列表
* 不得隐式绑定到任意 repo

## 6. 模块边界要求 (Module Boundary Requirements)

* 每个 runtime **SHOULD** 有独立的 state / actions / tests。
* 每个模块 **SHOULD** 有自己的 `AGENTS.md`，明确：
  * 唯一职责
  * 不能越过的边界
  * 最小验证命令
* 测试必须按模块就近放置；不得所有复杂集成测试都回流到一个总入口文件。

## 7. 与现有章节的映射 (Chapter Mapping)

* `04_storage.md`、`06_repository.md`、`07_diff_logic.md` → Authority Core / Projection & Repair
* `05_network.md`、`09_auth.md`、`16_web_thin_client_ledger.md` → Scope & Session Runtime / Document Runtime
* `03_rendering.md`、`08_ui_design*.md`、`11_i18n.md` → UI Shell & Feature Views
* `10_ai_agent.md`、`17_plugins.md` → Peripheral Systems

## 本章相关命令

* 无。

## 本章相关配置

* 无。

# 19_repo_refactor_blueprint.md - 仓库重构蓝图 (Repo Refactor Blueprint)

## 1. 目标

本蓝图用于把当前仓库逐步收敛为 infra-first 架构，使：

* 核心运行时边界清晰
* bug 能够定位到单一模块闭环
* 多人 / 多 agent 并行开发时不互相踩踏
* 核心功能优先于 plugin / AI / 外围扩展

本蓝图是 **Implementation Blueprint**，用于指导迁移顺序与目录调整，不得覆盖 `04/05/06/07/09/16/18` 的权威约束。

## 2. 当前结构的主要问题

### 2.1 Web：文件很多，但主干状态机仍不清晰

当前 `apps/web/src/hooks/use_core` 已拆成大量 `callbacks_* / effects_* / state_*` 文件，但仍然围绕一个总控 hook 运转。

表现：

* scope、pending、source control、sync、document lifecycle 高度耦合
* 新 bug 常常需要从 component -> callback -> effect -> protocol message 一路追踪
* runtime 边界更多是“文件前缀”而不是一级模块

### 2.2 Server：handler 和 runtime 装配边界不够显式

`apps/cli/src/server` 已经有很多 helper / support / selector 文件，但：

* server runtime assembly 仍然过厚
* projection / repair / scope recovery 分散在 startup、handler 与 support helpers 中
* 文档写路径、repo scope、source control 路径虽然可测，但边界不够一眼可见

### 2.3 Core：Authority 与 Projection 的目录边界还不够显式

`crates/core/src` 当前主要以 `ledger / sync / tree / source_control` 组织。方向基本正确，但：

* projection / repair 没有独立一级归属
* runtime 恢复路径经常要在 `sync + tree + server` 三处拼起来看

## 3. 目标模块版图 (Target Module Map)

### 3.1 crates/core

建议逐步收敛为以下一级职责：

* `authority/`
  * facts
  * append validation
  * authoritative query
  * runtime tables / indexes
* `projection/`
  * tree projection
  * workspace projection
  * materialize
  * repair / quarantine
* `scope/`
  * repo identity
  * branch identity
  * selector / resolution contracts
* `source_control/`
  * stage / commit / diff / history 的领域逻辑
* `protocol/`
  * client/server/sync message contract

说明：

* 不要求一次性改目录名，但职责必须逐步向上述布局靠拢。
* `projection` 与 `authority` 必须明确分层。

### 3.2 apps/cli

建议逐步收敛为：

* `server/runtime/`
  * AppState
  * startup / setup / router assembly
  * auth / native session assembly
  * plugin host API assembly
  * sync / watcher / tree assembly
  * metrics / prewarm / p2p peripheral assembly
  * static-file validation and router build admission
  * session lifecycle
* `server/handlers/document/`
  * open/edit/create/delete
  * doc ack/reject contract
* `server/handlers/scope/`
  * repo switch
  * branch switch
  * scope recovery
* `server/handlers/source_control/`
  * changes / diff / history / commit
* `server/services/projection_repair/`
  * startup scan
  * degraded repo handling
  * repair execution

### 3.3 apps/web

建议逐步收敛为：

* `runtime/session_client/`
  * ws lifecycle
  * reconnect
  * auth/session gating
* `runtime/scope_client/`
  * repo scope
  * branch scope
  * scope prefs
  * stale scope recovery
* `runtime/document_client/`
  * open doc
  * pending ops
  * ack/reject
  * navigation guard
* `runtime/source_control_client/`
  * staged/unstaged
  * diff sessions
  * graph/history
* `runtime/rendering_client/`
  * Markdown / editor bridge
  * render hints
  * CodeMirror / KaTeX object adapters
* `features/`
  * editor
  * explorer
  * source_control
  * mobile_shell

说明：

* 当前 `use_core` 目录不必立刻消失，但必须降级为 composition root，逐步按 client runtime 切分，而不是继续扩展 `effects_*` 前缀家族。
* Web client runtime 只能编排 UI intent、transport state、pending overlay 与 Object Plane adapter，不得拥有 ledger/source-control authority。
* UI 组件只能通过 typed handle 消费 client runtime，不得直接读写跨 runtime 的混合 signal 集合。

## 4. 优先迁移顺序 (Migration Order)

### Phase A — Projection & Repair

优先级最高。因为它直接影响：

* 文件树是否显示
* repo 启动是否可用
* fallback 是否长期污染正常路径

目标：

* 把 repair 从 scattered fallback 收成正式服务
* 明确 degraded / quarantined repo 的行为

Server startup 同步开始收敛到 `server/runtime/`：当前入口接收已绑定的
`start_server_with_bound_listener`，端口绑定归 command/native launcher，
server start 只保留 listener handoff、顺序编排、serve 与错误传播；
auth、plugin、sync、watcher、tree、metrics、static/router 装配必须迁入
runtime parts。

### Phase B — Document Pending / Ack / Reject

第二优先。因为它直接影响：

* 写入是否可信
* “未确认本地写入”是否误报
* Web thin-client 是否符合 `16`

目标：

* 形成独立 document runtime
* 明确 `Waiting / Rejected / Committed / WritebackFailed`

### Phase C — Repo Scope Runtime

第三优先。因为它直接影响：

* repo/branch 切换
* stale scope 恢复
* readonly / spectator 边界

### Phase D — Source Control Runtime

第四优先。因为它依赖 scope 与 document runtime 稳定之后才能真正收口。

### Phase E — Markdown / Rendering Runtime

第五优先。把现有 Markdown 体验补完，但不新增超前模式；优先完善 plan 中已经存在的 Source-first / Cursor Reveal / Widget 行为。

目标：

* 将 `index.html` 中的 editor/mobile/rendering 全局桥接迁入 `runtime/rendering_client` 对应的 JS bridge。
* `window.*` 只作为短期 compatibility surface，由单一 registry 集中挂载。
* DOM、CodeMirror、KaTeX 只属于 Object Plane adapter，不得清理 pending 或决定写入成功。

### Phase F — Peripheral Systems

最后处理：

* AI
* plugins
* external agent bridge

它们只能在核心链稳定后继续推进。

## 5. 文件迁移判断规则

当一个文件满足以下任意条件时，应考虑迁移或重组：

* 同时读写多个 runtime 的内部状态
* 既做协议路由又做业务恢复
* 既做 UI 决策又做 authority/path repair
* 只能通过命名约定看懂职责，而不是通过目录层级看懂

## 6. 模块完成标准

一个 runtime / infra 模块只有在满足以下条件后，才算重构完成：

* 有独立 state / actions / tests
* 上层只能通过 typed API 调用它
* 它的失败模式与恢复路径写入 plan
* 它有对应的 Chrome MCP 验证路径或集成测试入口

## 7. 推荐的验证方式

### 7.1 Core / Server

* `cargo test -p deve_core ...`
* `cargo test -p deve_cli ...`
* `cargo check -p deve_cli`

### 7.2 Web Runtime / UI

* `cargo test -p deve_web ...`
* `cargo check -p deve_web`
* Chrome MCP 实测对应页面路径

### 7.3 Chrome MCP 必测路径

* repo 启动与 scope 恢复
* 文档打开 / 编辑 / 离开页面
* 文件树显示
* source control stage / diff / history / readonly
* 移动端 drawer / outline / source control 交互

## 8. 当前主线之外的内容

以下内容不属于当前主线，不得阻塞核心 infra 重整：

* plugin runtime 扩展
* AI 工具调用增强
* 额外的视觉模式创新
* 不影响 authority / scope / pending write 的外围集成

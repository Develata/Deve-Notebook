# Runtime Convergence Audit — 2026-05-28

> 非权威 gap 分析(`docs/report/` 切片)。把 `docs/registry/runtime-skeleton-registry.md`
> 的当前承载状态对照 `docs/tasks/18_infra_runtime.md`(责任带)与 `docs/tasks/19_repo_refactor_blueprint.md`
> §3 目标模块版图 / §4 迁移顺序,量化"代码侧 runtime 收敛"缺口,为深入代码定起点。
> 权威以 plan/ + tasks/ + registry 为准;本报告只做快照与优先级建议。

## 1. 基线

- `cargo check --workspace`: 通过(exit 0);唯一告警 = `apps/web/src/api/native_http.rs:47`
  `packaged_shell_loopback_http_base_for_hostname` 未使用(无害 dead-code)。
- `cargo test --workspace`: 通过(exit 0);716 passed / 0 failed / 1 ignored。
  注:`apps/web` 用原生 `#[test]`(696 个,无 `wasm_bindgen_test`),会在 `cargo test`
  下运行 → web 写链重构**有原生测试网**(Phase B 可逐步 `cargo test` 验证)。
- 文档/治理门禁(本会话已全绿): `plan-coverage.sh`(blocking:0 / reverse-coverage OK /
  md-links 242 / metadata 32)、`check-architecture-registry.sh`(72 flows)、
  `check-graph-baseline.sh`、`check-acceptance-bindings.sh`(0 unbound)。

## 2. 收敛缺口总览

Registry 34 个 runtime 的状态:

| 状态 | 数量 |
|---|---|
| 已收敛 | 8 |
| 部分承载 | 21 |
| 抽象分层 | 4 |
| 未启动 | 1 |

→ 核心 authority/projection/repair/repo-scope 链已收敛;**transport / auth / web 写链 / source-control 周边 / backup / UI** 仍分散。

## 3. 按目标区的结构 delta

目标版图见 `tasks/19` §3。当前实际结构对比:

### 3.1 crates/core — delta 低(逻辑已收敛,目录布局未到位)

- 已收敛 runtime 的**逻辑**已成独立命名模块(`authority_storage` / `projection_persistence` /
  `repair` / `projection_repair` / `repo_catalog` / `repo_scope` / `source_control`),
  但仍挂在 `ledger/`、`sync/` 下,未提升到目标顶层 `authority/` `projection/` `scope/`。
- `tasks/19` §3.1 note 明确"不要求一次性改目录名,职责靠拢即可" → **低优先,可后置**。
- 唯一仍散:`watcher_runtime`(`sync/watcher/` + `watcher.rs` 两处)。

### 3.2 apps/cli/server — delta 中

- ✓ `handlers/document/`、`handlers/source_control/` 已对齐。
- ✗ 无 `handlers/scope/`:scope 散在 `repo_scope/` + `handlers/switcher/` + `handlers/repo/`。
- ✗ 无 `server/runtime/` 带:AppState / startup / setup 散在 `start/`、`session/`。
- ✗ 无 `server/services/projection_repair/`。
- ⚠ 结构异味:约 18 个 `*_test/` 目录混入 `server/` 模块树
  (`repo_scope_test`、`source_control_*_test`×7、`switcher_*_test`×4、`listing_*_test`×4、
  `*_scope_*_test` 等),违反 `tasks/19` §5"靠目录层级看懂职责"。→ **独立快速清理收益**。
- 部分承载: session / auth_gateway / transport / repo_scope_sync / relay_proxy / merge / diff_session。

### 3.3 apps/web — delta 最高(= Phase B 目标区)

- ✗ **完全没有 `runtime/` 带**;全部在 `hooks/use_core/` 下,且 `callbacks_*` / `effects_*`
  前缀家族泛滥(callbacks / callbacks_build / callbacks_sc / callbacks_sc_scope /
  callbacks_sc_target / callbacks_switch / callbacks_sync / effects / apply)——
  正是 `tasks/19` §3.3 警告的反模式。
- 部分承载且最散:**browser_document / pending_overlay / write_confirmation**(写确认链)、
  browser_peer / transport / browser_auth / render_projection / widget_bridge / outline_projection。
- 目标:`runtime/{session,scope,document,source_control}` + `features/`。

## 4. 收敛优先级(对齐 tasks/19 §4 迁移顺序)

1. **Phase B — Document Pending/Ack/Reject 写确认链(最高价值,第一刀)**
   - 收敛 `browser_document` + `pending_overlay` + `write_confirmation` + `document_runtime`
     到 `apps/web/src/runtime/document/`;CLI 侧 `handlers/document/` 固化 ack/reject 合同。
   - 价值:直接保障"未确认本地写入不误报"(可信性核心);状态机 `Waiting / Rejected /
     Committed / WritebackFailed`。有 `tasks/20_web_thin_client_ledger_migration.md` 专用蓝图。
   - 当前散落(registry):`web/editor/sync/`、`web/editor/hook_runtime.rs`、
     `web/hooks/use_core/pending/`、`web/editor/sync/history_replay.rs`、
     `web/hooks/use_core/callbacks_sync/write.rs`、`web/editor/sync/history_resend.rs`、
     `cli/server/handlers/document/`。
2. **CLI `*_test/` 树清理**(低风险、独立收益):把 server 模块树里的 `*_test/` 目录归位。
3. **crates/core 顶层目录重组**(authority//projection//scope/)——逻辑已收敛,**最低优先**。

## 5. 每个 runtime 的收敛完成标准(tasks/19 §6)

一个 runtime 只有满足以下才算收敛完成:

- 独立 state / actions / tests;
- 上层只能通过 typed API 调用;
- 失败模式与恢复路径写入 plan(并带 `//! plan_ref:` 注解,受 reverse-coverage 约束);
- 有对应 Chrome MCP 验证路径或集成测试入口。

> 每个 runtime 的当前承载路径以 `docs/registry/runtime-skeleton-registry.md` 为准;
> 收敛动作落地后必须同步更新该 registry 的 `status` 与 `current_module_path`。

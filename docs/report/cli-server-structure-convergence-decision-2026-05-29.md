# CLI Server 结构收敛裁定 — 2026-05-29

> §8 变更治理分析报告(`00_engineering_constitution.md` §8 要求:骨架级变更前出
> 分析报告 + USER 批准)。针对 `docs/report/runtime-convergence-audit-2026-05-28.md`
> §3.2 列出的 `apps/cli/src/server/` 结构缺口(`handlers/scope/` 收敛、
> `server/runtime/` 带、`server/services/projection_repair/`)逐项核对代码后裁定。
> 权威以 `docs/plan/` + `docs/tasks/` + registry 为准;本报告只做裁定与依据记录。
>
> **Status: USER 已裁定(2026-05-29,Opt 1)。** 配套 web 侧裁定见
> `docs/report/web-runtime-band-convergence-decision-2026-05-29.md`。

## 1. 背景与提案

审计 §3.2 把 cli server 标为「delta 中」,列出三个目标版图缺口:

1. **无 `handlers/scope/`**:声称 scope「散在 `repo_scope/` + `handlers/switcher/` + `handlers/repo/`」。
2. **无 `server/runtime/` 带**:AppState / startup / setup 散在扁平模块。
3. **无 `server/services/projection_repair/`**。

提案:是否把上述结构按 `tasks/19` §3 目标版图物理收敛。本报告对每项**先核代码、再裁定**,
避免重复审计快扫时的乐观偏差。

## 2. 当前结构(证据)

逐文件核对(见 `apps/cli/src/server/`):

- `repo_scope/`(13 文件:bootstrap/cleanup/counterpart/error/lookup/remote/resolve/selector/
  sync/sync_bootstrap/workspace + mod)——**已是内聚的 scope 解析模块**,
  `mod.rs` 暴露窄 typed API(`ResolvedRepo`、`resolve_session_repo`、
  `resolve_session_repo_and_sync`、`resolve_local_counterpart_repo`、workspace 写门控等),
  带 `//! plan_ref: 04_repository#repo-scope-runtime`。
- `handlers/switcher/switcher_scope.rs` / `switcher_selector.rs`——
  顶部 `use crate::server::repo_scope::{...}` **消费** `repo_scope` 的 typed API,
  并复用 `state.repo`(catalog runtime)的共享原语;**不重复实现解析**,
  只承载 switch 专属编排。
- `session/scope.rs`——`impl WsSession` 的**会话状态**读写
  (`scope_nonce` / `active_db` / `writer_identity` / `bind_repo`),属对象面状态层。
- `state.rs`(AppState)、`start.rs`、`setup.rs`、`launch.rs`、`prewarm.rs`、
  `node_role*.rs`——以扁平兄弟模块挂在 `server/` 下(共 27 个兄弟),
  `mod.rs` 以 `pub use` 暴露 `AppState` / `start_server*` / `ServerLaunchOptions`。
- `projection_repair`——`grep -r projection_repair apps/cli/src` 仅命中 `test_modules.rs`
  (测试模块名);**cli 侧无 projection_repair 生产代码**,逻辑全在
  `crates/core/src/sync/projection_repair_runtime.rs`(registry 标 `已收敛`)。
- 测试散落残留:`handlers/switcher/` 仍混着独立 `*_test.rs`
  (`switcher_selector_single_remote_test.rs`、`switcher_last_local_repo_test.rs`、
  `switcher_switch_nonce_test.rs`)。ec1ef1cf 只清了 `server/` 顶层 `#[path]` 测试。

## 3. 逐项裁定

### 3.1 `handlers/scope/` 收敛 → **REJECT(假缺口)**

审计的「scope 散在三处」是**表面命名观察**,不是真内聚问题。三处是**三个不同关切**,
已正确分离:

| 位置 | 关切 | 关系 |
|---|---|---|
| `repo_scope/` | scope **解析**(session hint → 可执行 selector / `ResolvedRepo`) | 单一权威,typed API |
| `switcher_*` | switch **目标**选择 + 编排(`select_target_repo`:URL 匹配 / 显示冲突回退 / 单远端 fallback) | 消费 `repo_scope` + `state.repo` |
| `session/scope.rs` | 会话**状态**读写 | 对象面,被 `repo_scope` 读取 |

两个 selector 回答的是**不同问题**:`repo_scope/selector.rs::resolve_repo_name_from_session`
= 解析「当前」session 的 repo;`switcher_selector.rs::select_target_repo`
= 解析「切到哪个」目标 repo。共享的底层查找早已沉到 `state.repo`。

→ 并进 `handlers/scope/` 会把三个关切强行揉到一起、**降低内聚**,违反「high cohesion,
low coupling」与 §1.2。**不做**。`repo_scope/` 即 scope 关切的收敛终态(只是不在
`handlers/` 名下,而这正是 `tasks/19` §3.1 note「不要求改目录名、职责靠拢即可」的范围)。

### 3.2 `server/services/projection_repair/` → **REJECT(空缺口)**

cli 侧无对应生产代码可归(§2 grep 证据)。projection_repair 逻辑在 core 且已收敛。
建一个空壳 services 带只会制造「为满足表格而创建空模块」(registry Notes 明令禁止)。**不做**。

### 3.3 `server/runtime/` 带 → **DEFER(#4-only,与 §1.3 张力)**

把 `state`/`start`/`setup`/`launch`/`prewarm`/`node_role*` 等生命周期模块从扁平兄弟
归进 `runtime/` 子目录,**确有**「靠目录看懂职责」的收益,且 cli 非响应式 → 正确性风险低
(835 测试是可靠回归网;re-export 保 `crate::server::{AppState, start_server, ServerLaunchOptions}`
公共路径不变)。

但:它是**纯 #4 可维护性**收益,且要移动 `AppState` 脊柱(全 handler 都持 `&Arc<AppState>`),
属 §1.3 明指的「局部优化」。其 risk/value profile 与刚 §8 DEFER 的 web 头段、core 顶层重组**同类**。
按统一精度裁定 → **DEFER**,等 §5 证伪点触发。

> 与 web 头段的差异已记录:cli 无响应式时序风险、无 `runtime→hooks` 依赖倒挂问题,
> 所以**风险更低**;若未来要做,它比 web/core 更易安全落地。但「低风险」≠「值得现在做」。

### 3.4 残留 `*_test.rs` → **NO-OP(经代码核对修正:idiomatic 就地测试,非结构异味)**

> 本项初判 PROCEED;读取 5 个文件后**修正为 NO-OP**——验证优先于盲目执行。

`handlers/{switcher,document,source_control}/` 里的 `*_test.rs` 经核对是**就地单元/集成测试**,
以 `super::` 够到父模块的 `pub(super)` 内部项或同级 test 辅助:

| 文件 | 够到的目标 | 可否搬 |
|---|---|---|
| `switcher/switcher_selector_single_remote_test.rs` | `super::switcher_selector::select_target_repo`(**`pub(super)`**)+ `super::switcher_prepare_test::build_state` | 否——搬成 `server` 扁平子模块后 `super`=`server`,够不到 `pub(super)` |
| `document/snapshot_delta_guard_test.rs` | `super::snapshot_delta_guard::{...}`(handler 内部项) | 否——同上 |
| `source_control/diff/remote_test.rs` | `super::remote_content` + `super::remote_test_support`(同级) | 否——依赖 diff/ 同级模块 |
| `switcher/switcher_switch_nonce_test.rs` | `super::handle_switch_branch`(pub)+ `crate::server::switcher_test_support` | 可(改 import),价值边际 |
| `switcher/switcher_last_local_repo_test.rs` | `super::handle_switch_branch`(pub) | 可(改 import),价值边际 |

ec1ef1cf 处理的异味是 **server 顶层** `*_test/` 目录污染模块树(已清);这些是**嵌套于 handler 的
就地测试**,与被测代码同处、走 `super::` 够内部项——是 Rust 标准 co-location,**不是异味**。
强搬会:(a) 断 `pub(super)` 可见性(或被迫放宽生产可见性),(b) 把测试与同级辅助拆散,
(c) 重蹈 `#[path]` 子模块解析坑。

→ **不搬**。其中 2 个走 public API 的 switcher 测试理论上可迁中央 `tests/switcher/`,
但属边际 #4 收益且要拆 import,按 §1.3「结构稳定优先于局部优化」**不主动做**;
若未来 switcher 集成测试集中度成为明确目标再议。

## 4. §3 优先级裁定

- scope 合并:不是收敛,是**反收敛**(降内聚)→ 与 #4 都不沾,直接 REJECT。
- projection_repair:无物可归 → REJECT。
- `server/runtime/` 带:#4 maintainability vs §1.3 结构稳定 + #1 正确性中性偏好 → **#1/§1.3 优先,DEFER**。
- 测试:经核对是 idiomatic 就地测试(够 `pub(super)` 内部项),非异味 → **NO-OP**(强搬反降质)。

## 5. 证伪点(`server/runtime/` 带何时重启)

任一触发即按 §8 重启 `server/runtime/` 带评估(届时先补 §5+§6 工程蓝图):

1. 新增 server 启动/生命周期模块使扁平兄弟继续膨胀(如 >30),目录已无法「靠层级看懂职责」。
2. 出现真实 bug 需跨 `state`/`start`/`setup`/`launch` 多文件追踪而现结构明显拖慢定位。
3. 桌面/移动 shell 复用 server 启动路径时,扁平结构成为复用障碍。
4. blueprint 升级为「强制」cli `server/runtime/` 目标(目前 `tasks/19` §3.1 note 明确不强制)。

scope / projection_repair 的 REJECT **无重启条件**——它们是结构判断,不是时机问题。

## 6. 验证

本裁定**零代码变更**(§3.1/3.2 REJECT、§3.3 DEFER、§3.4 NO-OP),仅产出本报告 +
registry Notes 一条。验证 = 文档门禁 `plan-coverage.sh` 全绿(registry path/status 无漂移、
md-links 无断链)。未触生产代码,既有 835 测试不受影响。

## 7. 不变更后果

- scope / projection_repair 不动:零负面(它们本就是正确终态)。本报告的价值在**钉死结论**,
  防后续 agent 看审计 §3.2 又去尝试合并而引入降内聚的 churn。
- `server/runtime/` 带不建:server 根目录维持 27 个扁平兄弟,新人/agent 首次定位生命周期模块略慢——
  可接受,且 AGENTS.md 已分类说明。
- 测试归位若不做:`handlers/switcher/` 生产与测试文件继续混放,违 §5,但无正确性影响。

## 8. 裁定结果

**USER 已裁定(2026-05-29):采纳 Opt 1。** 执行中核对发现残留测试为 idiomatic 就地测试,
「清理」据证据**修正为 NO-OP**(§3.4)——故本裁定最终**零代码变更**,纯治理产出。

- scope 合并 → REJECT(假缺口,§3.1)
- `server/services/projection_repair/` → REJECT(空缺口,§3.2)
- `server/runtime/` 带 → DEFER(§3.3,证伪点见 §5)
- 残留 `*_test.rs` → NO-OP(§3.4,经代码核对为 idiomatic 就地测试,强搬反降质)

> 本裁定未请独立 agent 评审(stakes 低于 web 主链决策,且为「假缺口」结构判断);
> 如需 Codex 复核可后置补做。

## 附录:registry 影响

- 不新增/不改 runtime 名称与 status。
- `repo_scope_sync_runtime` / `session_runtime` / `auth_gateway` 维持 `部分承载`——
  其分布是**按关切有意分离**,非收敛缺口;本报告为该判断的依据档。
- registry Notes 增一条指向本报告,防 cli 结构「假缺口」被重复尝试。

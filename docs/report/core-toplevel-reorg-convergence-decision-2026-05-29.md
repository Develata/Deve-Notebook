# crates/core 顶层重组收敛裁定 — 2026-05-29

> §8 变更治理分析报告(`00_engineering_constitution.md` §8 要求:骨架级变更前出
> 分析报告 + USER 批准)。针对 `docs/report/runtime-convergence-audit-2026-05-28.md`
> §4 第 3 项(最低优先)与 §3.1 的 `crates/core` 顶层目录重组
> (`authority/` `projection/` `scope/`)逐项核对代码后裁定。
> 权威以 `docs/plan/` + `docs/tasks/19` §3.1 + registry 为准;本报告只做裁定与依据记录。
>
> **Status: USER 已裁定(2026-05-29,Opt A — DEFER + NO-OP,净零代码,关闭收敛治理)。** 配套裁定见
> `docs/report/web-runtime-band-convergence-decision-2026-05-29.md`(web 头段 DEFER)与
> `docs/report/cli-server-structure-convergence-decision-2026-05-29.md`(cli 结构 REJECT/DEFER/NO-OP)。
> 本报告完成后,runtime 收敛路线图三大剩项(web/cli/core)全部经 §8 裁定。

## 1. 背景与提案

审计 §3.1 把 `crates/core` 标为「delta 低(逻辑已收敛,目录布局未到位)」,§4 列为
**最低优先**剩项。蓝图 `tasks/19` §3.1 给出 core 目标一级版图:

- `authority/`(facts / append validation / authoritative query / runtime tables）
- `projection/`(tree / workspace projection / materialize / repair / quarantine）
- `scope/`(repo identity / branch identity / selector / resolution contracts）
- `source_control/` / `protocol/`

提案:是否把已收敛的 `*_runtime` 模块从现挂载点(`ledger/manager/`、`sync/`)物理提升到
上述顶层目录。本报告**先核代码、再裁定**,沿用 cli 裁定的「验证优先于盲目执行」纪律。

## 2. 当前结构(证据)

### 2.1 已收敛 runtime 全是专用命名模块(registry 证据)

| runtime | status | 当前路径 |
|---|---|---|
| `authority_storage_runtime` | 已收敛 | `ledger/manager/authority_storage_runtime.rs` |
| `projection_persistence_runtime` | 已收敛 | `sync/projection_persistence_runtime.rs` |
| `projection_repair_runtime` | 已收敛 | `sync/projection_repair_runtime.rs` |
| `repo_catalog_runtime` | 已收敛 | `ledger/manager/repo_catalog_runtime.rs` |
| `repo_scope_runtime` | 已收敛 | `ledger/manager/repo_scope_runtime.rs` |
| `source_control_runtime` | 已收敛 | `ledger/manager/source_control_runtime.rs`; `source_control/` |
| `watcher_runtime` | 部分承载 | `sync/watcher/`; `watcher.rs` |

每个已收敛 runtime 都是**单文件专用模块**,职责边界清晰、registry 标 `已收敛`。审计原文亦确认
「核心 authority/projection/repair/repo-scope 链**已收敛**」。

### 2.2 消费方不按 `_runtime` 路径引用

`grep` `apps/` 对上述 `*_runtime` 模块名:**仅命中 1 处**(`apps/cli/src/server/test_modules.rs`
的测试模块名)。即 cli/web **不直接 import 这些 runtime 的深路径**,而是经
`deve_core::{ledger, sync, source_control}` 父模块的 typed API 消费。runtime 文件是父聚合的
内部组织单元,公共面由父 `mod.rs` re-export。

### 2.3 `ledger/manager/` 是一个真实聚合

`ledger/manager/` 同时持有 `authority_storage_runtime`、`repo_catalog_runtime`、
`repo_scope_runtime`、`source_control_runtime`,以及 `locator` / `repo_lookup` /
`remote_repo_scan` / `projection_locator` / `projection_cleanup` /
`source_control_target_resolution` 等协作模块——它们因「同属 LedgerManager 编排」而内聚。

### 2.4 `watcher.rs` 是契约 facade,非散落

`crates/core/src/watcher.rs` 全文 9 行:`pub use crate::sync::watcher::{WatcherError,
start_repo_watcher, stop_repo_watcher}` + `validate_watch_root`,带
`//! plan_ref: 03_storage/watcher#watcher-contract`。即顶层 `watcher` = **存储层 watcher 公共契约
锚点**(受 reverse-coverage 约束),实现体在 `sync/watcher/`(dispatch/filter/registry)。
registry 标 `部分承载` 正记录这层 facade/impl 分层。

## 3. 逐项裁定

### 3.1 顶层 `authority/` `projection/` `scope/` 重组 → **DEFER(#4-only,与 §1.3 张力;蓝图明确不要求一次性改)**

蓝图 §3.1 的**实质要求**是两条:
1. 「`projection` 与 `authority` 必须明确分层」;
2. 「projection / repair 有独立一级归属」(§2.3 痛点)。

核对结论:**这两条实质要求在模块层已满足**——`authority_storage_runtime` 与
`projection_persistence_runtime`/`projection_repair_runtime` 已是分离的专用 `已收敛` 模块,
authority 与 projection 的边界在命名与 registry 上一眼可见。剩余 delta **仅是把它们提级为
顶层目录名**,而蓝图同段**明确写「不要求一次性改目录名,职责逐步靠拢即可」**。

该提级是**纯 #4 可维护性**收益,且代价真实:

- **拆散 `ledger/manager/` 聚合**(§2.3):把 `authority_storage_runtime`→`authority/`、
  `repo_scope_runtime`→`scope/`、`projection_*`→`projection/` 提出后,LedgerManager 的协作
  成员被分散到 3+ 顶层目录,其编排内聚反而下降——与「high cohesion」张力。
- **动最高正确性权重 crate 的模块路径**(§3 #1):core 是协议/模型/账本权威层,其路径churn
  风险评估应严于 cli;即便有 re-export 兜底,也要新建蓝图未要求的兼容 shim 层。
- **profile 与已 DEFER 的 web 头段、cli `server/runtime/` 带同类**:纯 #4 vs §1.3 结构稳定。
  按统一精度,core(正确性权重更高)更应 DEFER。

→ 它**不是假缺口**(蓝图确实以此为目标),但是**当前低价值 + 真实 §1.3 代价 + 蓝图明示可后置**
的项 → **DEFER**,等 §5 证伪点触发。

### 3.2 `watcher_runtime` 合并 → **NO-OP(契约 facade,非散落)**

`watcher.rs` / `sync/watcher/` 的双处分布是**有意的契约 facade vs sync 实现分层**(§2.4),
非低内聚。`部分承载` 是该分层的如实记录,同 cli `repo_scope_sync_runtime` 的「按关切有意分离」。

合并会:(a) 删顶层 facade 则需把 `03_storage/watcher#watcher-contract` 的 `//! plan_ref:`
锚点迁到 `sync/watcher/mod.rs`(reverse-coverage 风险),(b) 把存储层公共契约面与 sync 内部
dispatch 实现揉到一处,丢失「契约可独立定位」。为边际 #4 收益不值得 → **不动**。

## 4. §3 优先级裁定

- 顶层目录重组:实质分层已达成,剩纯目录提级 → #4 maintainability vs §1.3 结构稳定 +
  最高 #1 正确性权重 → **§1.3/#1 优先,DEFER**。
- watcher 合并:契约 facade 分层,合并损契约可定位性 → **NO-OP**。

## 5. 证伪点(顶层重组何时重启)

任一触发即按 §8 重启评估(届时先补 §5+§6 工程蓝图):

1. **分层被实际违反**:新功能让 authority 逻辑渗入 projection(或反向),模块层边界不再足以
   守住蓝图「必须明确分层」——此时用物理顶层目录强制分层有真实价值。
2. **§2.3 痛点恶化**:runtime 恢复路径需跨 `sync + tree + server` 拼读,且现结构明显拖慢真实
   bug 定位。
3. 新增已收敛 runtime 在现 `ledger/sync` 布局**无自然归属**,确需顶层 `authority/projection/scope/`
   才能安放。
4. blueprint 把 core 顶层重组从「不要求一次性改」升级为**强制**目标。

watcher NO-OP **无重启条件**——它是结构判断(facade 分层正确),非时机问题。

## 6. 验证

本裁定**零代码变更**(§3.1 DEFER、§3.2 NO-OP),仅产出本报告 + registry Notes 一条。
验证 = 文档门禁 `plan-coverage.sh` 全绿(registry path/status 无漂移、md-links 无断链)。
未触生产代码,既有 core 测试套件(80+ 测试文件)不受影响。

## 7. 不变更后果

- 顶层重组不做:core 维持 `ledger/sync/source_control/...` 现布局,首次定位 authority/projection
  runtime 需知其挂在 `ledger/manager/`、`sync/` 下——可接受,且 registry 已逐 runtime 标注路径,
  authority/projection 分层在命名层已可见。本报告价值在**钉死结论**,防后续 agent 看审计 §3.1
  又去做提级而拆散 LedgerManager 聚合的 churn。
- watcher 不合并:零负面(facade 分层是正确终态)。

## 8. 裁定结果

**USER 已裁定(2026-05-29):采纳 Opt A — DEFER + NO-OP(零代码变更),关闭 runtime 收敛三大剩项的 §8 治理。**

- 顶层 `authority/`/`projection/`/`scope/` 重组 → DEFER(§3.1,证伪点见 §5)
- `watcher_runtime` 合并 → NO-OP(§3.2,契约 facade 分层正确)

> 备选 Opt B(未采纳):USER 明确不顾 §1.3、要求**现在物理执行**顶层提级——则本报告升级
> 补 §5/§6 工程蓝图(逐 runtime 迁移顺序 + re-export 兼容层 + 回退验证)后落地。
> 本裁定使 web/cli/core 三大剩项全部经 §8 裁定完毕,runtime 收敛路线图无未裁定剩项。
>
> 本裁定未请独立 agent 评审(DEFER/NO-OP 为保守方向,stakes 同 cli 裁定;蓝图本就明示可后置);
> 如需 Codex 复核可后置补做。

## 附录:registry 影响

- 不新增/不改 runtime 名称与 status。
- `watcher_runtime` 维持 `部分承载`——其分布是 facade/impl 有意分层,非收敛缺口;本报告为该判断的依据档。
- registry Notes 增一条指向本报告,防 core 顶层「提级」被重复尝试而拆散 `ledger/manager/` 聚合。

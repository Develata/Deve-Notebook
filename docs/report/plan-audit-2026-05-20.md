<!-- Generated: 2026-05-20 -->

# Deve-Notebook 工程蓝图审计报告 — `docs/plan/`

- **审计范围**：`docs/plan/` 全部 19 章（约 5 767 行）+ `AGENTS.md` + `plugins/agent_bridge/01_agent_bridge.md`
- **审计层次**：战略级，结论先行
- **配套报告**：[docs-audit-2026-05-20.md](./docs-audit-2026-05-20.md)（待出）、[code-audit-2026-05-20.md](./code-audit-2026-05-20.md)（待出）

---

## 1. 总体结论

`docs/plan/` 已经达到一个**远高于一般个人项目**的蓝图密度：宪法、术语、Authority Core、Runtime Protocols、UI Shell、Peripheral 五层结构清晰，规范性用语 (MUST/SHOULD/MAY) 严格执行，双射机制 (`plan_ref` + `plan-coverage.sh` + 锚点注册表) 已落地。在「能否指导施工」这件事上，蓝图是合格甚至超前的。

但**蓝图的负载已经超出维护带宽**：

- 章节内反复出现「Refactor Target」「Runtime Boundary」「Forbidden Patterns」三类同质段落，27 个 runtime 名称散布在 8 个章节里，跨章一致性靠 `deve-note plan.md` 的 Runtime Skeleton Registry 兜底，但**没有任何机器可执行的字段表达 runtime 的「未收敛 / 收敛中 / 已收敛」状态**——蓝图把目标说清了，却没说现在在哪一档。
- 平台子章（08_02/03）把 `gate policy` 文字常量、状态机、子进程合同写进了 UI shell 章，事实上承担了 governance 决策记录。这不是错误，但**违反了 §00 "shell 不得携带 authority"** 的精神，应当部分迁移到 `15_release.md` 或专设 governance log 章节。
- Plan 与 code 的 `Primary Code Areas` 字段是「pinned-by-glob」（如 `apps/web/src/hooks/use_core/effects/handshake*.rs`），**没有任何机制保证这些路径还存在**；它的反向校验在 `scripts/plan-coverage.sh`，但 `Primary Code Areas` 字段本身没参与扫描。

总体判断：**Plan 已经过了「能不能造」的门槛，现在卡在「能不能持续维护」的门槛。** 后续重点应是去重、增加状态字段、收紧子章职责，而不是再往里塞内容。

---

## 2. 强项（保持现状）

| 项 | 评价 |
|---|---|
| 工程宪法 (`00`) | 9 条总则简明、不出错位、对 AI 执行者友好；§3 优先级表是同类项目稀缺资产。 |
| 术语篇 (`01`) | 用数学符号 (`L_r`, `Apply`, `Project`) + 中英对照，规范性用语首屏即定义；NodeId / DocId / Layer 三元关系形式化精确。 |
| 双射机制 (`AGENTS.md` §Layer 1-3) | `plan_ref` 注解 + 稳定锚点注册表 + `plan-coverage.sh` 反向覆盖矩阵 + 验收用例命名约束 (`ACC-XXX` ↔ `acc_xxx_<slug>`)，三层互锁，国内同规模项目几乎看不到。 |
| Runtime Skeleton Registry (`deve-note plan.md`) | 把分散在各章 §Refactor Target 的 27 个 runtime 集中登记，并显式给出主链 (`browser_peer_runtime -> browser_document_runtime -> pending_overlay_runtime -> write_confirmation_runtime`)，是跨章一致性的最后防线。 |
| Pending Overlay vs `pending_fs_ops` 边界 | `01 §定义` / `04 §4.3` / `07 §3.1` / `16 §2` 四章一致地拒绝混淆，是全篇最干净的硬约束。 |
| 错误码单一权威 (`11 §6`) | 明文「本节是唯一权威」+ 其他章节只允许「引用」，杜绝平行错误码目录的常见漂移。 |
| Git mirror 单一权威 (`07 §2.3.1`) | 同上，`07_diff_logic.md#git-mirror-lifecycle` 是 `.git/` 边界唯一来源，`12 §1` / `13` / `14 §1.3` 都正确地引用而非复写。 |
| Native packaging 门禁 (`14 §1.4`) | 把 `Tauri` 依赖、`native-packaging` feature、process adapter gate、Android/iOS shell-only execution gate 分四档，每档绑定 `CURRENT_*_POLICY` 常量；这是 plan 里少见的「字段化决策」。 |

---

## 3. 关键 Finding（按优先级）

### P0 — 影响后续工程的结构性问题

#### F-P0-1. 27 个 Refactor Target 没有「收敛状态」字段

**事实**：以下 runtime 名称在各章 §Refactor Target 出现：

- `04`: `authority_storage_runtime` / `projection_persistence_runtime` / `watcher_runtime` / `repair_runtime`
- `05`: `transport_runtime` / `repo_scope_sync_runtime` / `browser_peer_runtime` / `relay_proxy_runtime`
- `06`: `repo_catalog_runtime` / `repo_scope_runtime` / `projection_repair_runtime`
- `07`: `source_control_runtime` / `diff_session_runtime` / `merge_runtime`
- `08`: `ui_shell` / `application_control` / `feature_runtime`
- `09`: `session_runtime` / `auth_gateway` / `browser_auth_runtime`
- `03`: `document_runtime` / `render_projection_runtime` / `widget_bridge_runtime` / `outline_projection_runtime`
- `16`: `browser_document_runtime` / `pending_overlay_runtime` / `write_confirmation_runtime`

`deve-note plan.md` 的 Runtime Skeleton Registry 把它们罗列出来，但**只描述目标边界，不描述现状**。读者无法仅凭 plan 回答「`pending_overlay_runtime` 已经独立成模块了吗？还是仍混在 `use_core/pending*.rs` 里？」

**风险**：
1. 新 PR 提交时无法判断「该收敛到哪个 runtime」是否已经成立——`tasks/18_infra_runtime.md` 是迁移蓝图但未必持续更新。
2. 长期累积成「永远是目标但永远不到」的清单，与 §00 §8 「骨架级变更必须明确批准」精神冲突。

**建议**：
- 在 `deve-note plan.md` Runtime Skeleton Registry 表里增加 3 列：`status` (`未启动` / `迁移中` / `已收敛`)、`primary_module` (当前承载文件)、`tracking_task` (对应 `docs/tasks/`)。
- 任何 PR 让某 runtime 跨越 `迁移中 → 已收敛` 时，要求同步更新该表，作为 review checklist 项。
- `scripts/plan-coverage.sh` 增加一项「未在 Skeleton Registry 出现的 `plan_ref` chapter」warning。

#### F-P0-2. 平台子章承担了 governance 决策，违反层级原则

**事实**：`08_ui_design_02_desktop.md §1.3` 与 `08_ui_design_03_mobile.md §1.5` / §1.6 / §1.7 写入了 gate policy 决策内容：

```
CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY.decision =
    DesktopAndMobileDependencySpikeOpen
desktop_tauri_dependencies_allowed = true
mobile_tauri_dependencies_allowed = true
default_build_remains_no_tauri = true
authority_writes_allowed = false
```

这种字面常量本质上是「治理决策快照」，应当与 `14_tech_stack.md#native-packaging-dependency-gate` 配对，或者放到 `docs/report/` 的决策日志里。当前安排下，UI shell 章节承担了原本属于 `release/governance` 层的责任。

`08_ui_design.md §8.5 native-adapter-gate-registry` 已经定下规则：「Desktop/Mobile 子章 **MUST NOT** 引入新的 ledger、vault、source-control、sync、search 或 settings authority」。但 supervisor 状态机、`SpawnFailed → Offline` 分类、`session_material` MUST 约束等实际上是 `09_auth.md` 与 `15_release.md` 的延伸，仍然停留在 UI shell 章。

**建议**：
- 拆出 `docs/plan/14a_native_track_gates.md`（或并入 `14_tech_stack.md` §1.4 的 sub-anchor），承担 gate policy 与 supervisor contract。
- `08_ui_design_02/03` 只保留「Desktop/Mobile shell **delta**」段落：尺寸、手势、安全区域、单端 UX 差异，剩余 70%+ 内容迁移到 native track 章节。
- 当前 `08_ui_design_03_mobile.md` 372 行里前 192 行（§1）是 native adapter，§2 以下才是 mobile UI，这种比例本身就在提示。

#### F-P0-3. `Primary Code Areas` 字段未参与 plan-code 双射校验

**事实**：每个 plan 章都有 `Primary Code Areas` 列表，例如：

```
06_repository.md:
  apps/web/src/hooks/use_core/callbacks_switch_*.rs
  apps/cli/src/server/handlers/switcher*.rs
```

这些 glob 是「叙事性引用」——`plan_ref` 注解机制（代码侧 → plan 章节）有反向覆盖矩阵保证，但 `Primary Code Areas` 字段（plan 章节 → 代码 glob）**没有**任何脚本扫描这些 glob 是否仍能匹配到文件。

**风险**：随着代码重构（特别是上面 F-P0-1 提到的 runtime 收敛），这些 glob 会悄无声息变成空集，读者按 plan 找代码会扑空。

**建议**：
- `scripts/plan-coverage.sh` 增加正向校验：扫描每章 `Primary Code Areas` 下声明的 glob，若无任何文件匹配则输出 warning，超过 N 章命中 warning 即阻塞。
- 或者**直接放弃** `Primary Code Areas` 字段，只靠 `plan_ref` 反向矩阵作为唯一索引——这其实更符合 §00 「抽象不得建立在首版实现上」的精神，但需要 README 引导工具变更。

---

### P1 — 一致性与冗余

#### F-P1-1. Status 分类未统一

**事实**：`deve-note plan.md` 定义 5 类状态——Current MUST / Current UI Contract / Approved Runtime Architecture / Optional Product Layer / Planned/Optional。但实际章节 Metadata 出现的 `Status:` 值有：

| 章节 | 实际 Status |
|---|---|
| `00` | `Governing Rule` (不在分类内) |
| `14`, `15` | `Reference` (不在分类内) |
| `17` | `Deferred` (不在分类内) |
| `10` | `Optional Product Layer` ✓ |
| `16` | `Approved Runtime Architecture` ✓ |
| 其余 | `Current MUST` / `Current UI Contract` ✓ |

3 个章节使用了未登记的状态值。

**建议**：要么扩充 `deve-note plan.md` §文档状态与职责边界 列表为 8 类，要么把 `Governing Rule` 改名为 `Current MUST`、把 `Reference` / `Deferred` 改名为 `Planned / Optional`。前者更精确，后者更省维护。

#### F-P1-2. 编辑器 buffer 三元变量在 03 与 16 命名不一致

**事实**：

- `16_web_thin_client_ledger.md §2.1`：`L_confirmed` / `O_session` / `V_web`
- `03_rendering.md §2.1`：`L_confirmed` / `O_pending` / `V_editor`

同一概念三元组在两章用了 `O_session` vs `O_pending`、`V_web` vs `V_editor`。`16` 是 web thin client 章，`03` 是渲染章，物理上同一个对象，命名却分叉。

**建议**：在 `01_terminology.md` §2 增加 `O_session` 与 `V_view` 的权威别名（或反过来让 `03` 改名），并在两章引用同一个稳定锚点。

#### F-P1-3. `web` 章节中混入了 release/CI 内容

**事实**：`08_ui_design_01_web.md §1.1 构建流水线` 给出了具体的 trunk build 步骤、`include_bytes!` 实现选择、`http://127.0.0.1:<port>` smoke 入口约束、`apps/web/dist` 重建顺序。

这些是 release/CI 关注点，应当在 `15_release.md`。当前安排让「UI 设计章」承担了「分发与构建顺序」职责。

**建议**：§1.1 整段迁移到 `15_release.md`，`08_01` 只保留「Web 端 = single binary distribution 的 served target」的一句指针。

#### F-P1-4. `i18n` key reference 表覆盖不完整

**事实**：`11_i18n.md §5` 列了 source_control / common / header / sidebar / settings / bottom_bar / playback / command_palette / search 9 个命名空间的 key。但代码侧明显存在更多命名空间（auth 错误展示、diff 视图、merge conflict、AI chat、native adapter notice 等），这些章节在 plan 中明确引用 `t::xxx::yyy()` facade，却没在该表中登记。

读者无法仅凭 plan 回答「`t::auth::session_expired` 这个 key 存在吗？」

**建议**：
- 要么明文标注「§5 仅为示例，权威 keys 见 `apps/web/src/i18n/`」；
- 要么改成「权威表」，并由 `scripts/plan-coverage.sh` 增加扫描：所有 `t::namespace::key()` 调用必须出现在该表（缺失即阻塞）。

#### F-P1-5. 02 §3 Phase 0 描述已经过时

**事实**：`02_positioning.md §3 Phase 0: 核心验证原型` 写「在构建任何 UI 之前，**必须**先行构建并通过验证的纯命令行原型」。从 `docs/report/` 看（如 `full-regression-gate-refresh-after-local-target-host-smoke-closure-2026-05-20.md`），项目早已远超此阶段。

**建议**：把 Phase 0 改为「Phase 0 已闭合（参考报告 X）」一句话存档，避免新读者误以为仍在验证 reconciliation 鲁棒性。

#### F-P1-6. 08_02 / 08_03 大量结构性重复

**事实**：Desktop 与 Mobile 两章的「Minimal Native Adapter Contract」「Embedded Service Supervisor Contract」「Process Adapter Gate」「Packaging Scaffold」内容 70%+ 重复，差异点主要在 mobile 的 `BackgroundSuspended` / `ForegroundReprobe`。

`08_ui_design.md §8.6 native-post-gate-common-contract` 已经做了一次共享化，但只覆盖到「post-gate」阶段。pre-gate native adapter contract 没有共享化。

**建议**：把 §1 第二级标题（`Minimal Native Adapter Contract` / `Embedded Service Supervisor Contract` / `Process Adapter Gate`）抽出为 `08_ui_design.md` 的 §8.7~§8.9（pre-gate common），Desktop/Mobile 章只列 deltas（Desktop = 无；Mobile = background lifecycle）。

---

### P2 — 局部清理

#### F-P2-1. `17_plugins.md` Status `Deferred` 与现状不符

§2 明确允许保留**已经存在**的 Rhai runtime / `PluginCall` / `PluginResponse`。这是「现役兼容边界」，不是「未来 deferred」。建议拆成两章或显式分段标注：`§2 Active Boundary` vs `§3-4 Deferred Future`。

#### F-P2-2. `protocol_version = 9` 在 `05 §4.2` 硬写

每次 schema 变更都要改 plan + 同步收发端窗口。这本身无错，但建议在 plan 加一句「该数字由 `crates/core/src/protocol/version.rs` 单一来源同步」，并在 plan-coverage 校验中比对。

#### F-P2-3. `Primary Code Areas` 路径含 `apps/web/src/hooks/use_core/effects/handshake*.rs` 这样的具体子模块

如果 F-P0-1 的 runtime 收敛真的发生，这些路径会失效。可改为「以 `plan_ref` 反向矩阵为准；本字段仅为辅助导航」。

#### F-P2-4. AGENTS.md 锚点注册表缺少 owning chapter 校验

§1 锚点表 36 项，但没有「锚点应只出现在其所属章节」的脚本校验。`scripts/plan-coverage.sh` 应保证每个锚点正好存在于一个章节的 `{#anchor}` 标记中。

#### F-P2-5. 跨章引用的 `Web Write Layer (16 §11.2)` 与 `Source Control Server Runtime (07 §9.3)` 边界

两者都讲「server side write」，但承担不同语义（document edit vs source control commit）。建议在 `deve-note plan.md` Runtime Skeleton Registry 显式标注「这两个 runtime 不重叠：`document_runtime` 拥有 doc edit，`source_control_runtime` 拥有 stage/commit」。

---

## 4. 影响 / 不影响 工程的小项（不在上面列表）

- `00 §3` 优先级表若加 2 个典型冲突案例（例如「正确性 vs 性能」时如何选择）将更具操作性。
- `13_settings.md §1` 环境变量表里 `DEVE_BIND_ADDR` 出现在 `15 §5.2` 例子里但未在 13 列出。建议补登记。
- `15_release.md` 提到 `nightly.yml` 与 `speckit-sync-check.yml` 「不属于权威要求」——若仓库里仍残留这些 workflow 文件应同步清理（Phase 3 会查）。

---

## 5. 给后续 Phase 的输入

Phase 2 (`docs/` 审计) 重点关注：

1. `docs/features/` vs `docs/plan/` 是否真的只承担「what the product does」而不窃据「how it is engineered」。
2. `docs/acceptance-cases/` 是否每个 `ACC-XXX.md` 都有对应集成测试 + 是否被 `scripts/plan-coverage.sh` Layer 3 捕获。
3. `docs/report/` 84 个文件是「阶段性快照」还是「持续治理日志」——按 F-P0-2 的建议，部分内容应该回流到 plan 章节。
4. `docs/tasks/18 / 19 / 20` 蓝图是否还在用，与 plan §Refactor Target 是否对得上 F-P0-1 提的 27 runtime。
5. `docs/coverage-matrix.md` 是否实际维护、谁在用、是否与 `scripts/plan-coverage.sh` 输出对齐。
6. `docs/overview/architecture-doc.lisp` 等图形产物（lisp / dot / svg）是否仍准确，还是已成「装饰」。

Phase 3 (代码审计) 重点关注：

1. 文件大小红线：250 行软警告 / 500 行硬阻塞（`AGENTS.md` §最小强制检查 + 仓库根 `AGENTS.md`）。
2. `plan_ref` 注解的覆盖：`scripts/plan-coverage.sh` 实际输出。
3. F-P0-1 中 27 个 runtime 的真实承载位置——是否真的拆出独立模块。
4. F-P1-2 的 `O_session` / `O_pending` 在代码中实际是哪个命名。
5. `pending overlay` 与 `pending_fs_ops` 在代码里是否真的分属不同表 / 不同类型，watcher 是否真的不碰 overlay。
6. `Primary Code Areas` 字段所列 glob 是否还能命中文件（F-P0-3）。
7. Native adapter 默认构建是否真的无 `tauri` import（`scripts/check-native-track-boundary.sh` 是否 green）。

---

## 6. 数字摘要

- 章节数：19（+ AGENTS.md + plugins/agent_bridge/01）
- 总行数：5 767
- 最大章节：`04_storage.md` 514 行；`07_diff_logic.md` 442；`08_ui_design.md` 442
- 最小章节：`02_positioning.md` 78；`00_engineering_constitution.md` 92
- 引用唯一权威机制：i18n 错误码、Git mirror lifecycle、Native packaging gate、Authority bridge 各 1 处
- 锚点注册表登记数：36
- 显式 Refactor Target runtime 数：约 27（含部分跨章别名）
- 显式 `MUST` 出现次数（粗估）：>300
- Status 分类未对齐章节数：3（00 / 14-15 / 17）

---

## 7. 是否推荐进入 Phase 2

**是。** plan 层无阻塞 finding；上述 P0-P2 项都属于「可在不停工的情况下渐进修复」的治理债务，不影响 docs/ 与代码审计的展开。建议把 F-P0-1（runtime 收敛状态字段）作为后续治理批次的最高优先级，因为它直接决定了 Phase 3 代码审计能否给出可信的「现状 vs plan 漂移」结论。

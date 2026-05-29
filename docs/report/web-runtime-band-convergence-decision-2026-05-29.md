# Web Runtime 带收敛 — 骨架级变更治理分析报告(§8)

> 依据 `docs/plan/00_engineering_constitution.md` §8(变更治理)提交的骨架级变更分析报告,
> 供管理员 / USER 按 §8 批准路径裁定。本报告是**非权威分析**(`docs/report/` 切片),
> 不构成 plan/ 规范,也不构成 §5 工程蓝图;结论生效以 USER 明确批准为准。

## 元信息

- `提交日期`: 2026-05-29
- `状态`: USER 已裁定(2026-05-29)—— 裁定一:接受 DEFER(暂不物理重排 `effects/`);裁定二:批准施工 §11 两件小事。经独立双 agent 评审(见 §13)。
- `变更级别`: 骨架级(改主调用架构的模块边界)
- `触发`: `docs/tasks/19_repo_refactor_blueprint.md` §3.3(apps/web 目标版图)、`docs/registry/runtime-skeleton-registry.md`(`browser_peer_runtime` / `browser_document_runtime` 仍 `部分承载`)
- `依据`: 宪法 §1.3 / §2 / §3 / §5 / §6 / §7 / §8 / §9
- `相关`: `docs/report/runtime-convergence-audit-2026-05-28.md`、`docs/plan/09_web_thin_client_ledger.md`、Phase B(写确认链 steps 1-5,commits `60d5a94d`..`c45723dd`)

## 1. 变更提案(受审对象)

把 `apps/web/src/hooks/use_core/effects/` 下 ~30 个 `message_*` 派发/路由模块,按目标版图物理迁入新的 `apps/web/src/runtime/{session,scope,document,source_control}` 带,并把 `effects_*` / `message_dispatch_*` 扁平前缀改为按带组织;同步处理 `browser_document_runtime`(`editor/sync/` + `editor/hook_runtime.rs`)的归属。

这是 §1.2 意义上的「反向塑造骨架」候选动作:它不新增功能,而是重排既有主骨架的模块边界,故按 §8 走治理。

## 2. 原因(motivation)

- `tasks/19` §3.3 的 apps/web 目标版图要求收敛为 `runtime/{session,scope,document,source_control}` + `features/`,并明确"应逐步按 runtime 切分,而不是继续扩展 `effects_*` 前缀家族"。
- registry 中头段两 runtime 仍 `部分承载`;审计 §3.3 把 apps/web 列为 delta 最高区。
- `effects/` 当前以扁平前缀(`message_dispatch_route_projection` 等)承载职责,违反 `tasks/19` §5「靠目录层级看懂职责」与宪法 §5「目录层级可读」精神。

## 3. 架构现状(事实基础)

2026-05-29 调研结论:

- `effects/message_dispatch.rs::handle_message` 是干净的**分层责任链**:`route_projection_and_sync` → `route_runtime` → `route_control` → `route_protocol_and_write` → `handle_sc_or_remaining`,每层返回 `Option<ServerMessage>`。背后 ~30 个 `message_*` 处理模块按关注点分组。
- `editor/sync/mod.rs::handle_server_message` 是同构小链(`route_doc` → `route_payload`),作用在 `SyncContext`,内聚、命名清晰。
- `editor/hook_runtime.rs::EditorRuntime` 紧绑 `EditorContext`,本质是编辑器特性状态,非 sync runtime。
- 测试网:apps/web 用原生 `#[test]`(全仓 706 个,其中 `effects/` + `editor/sync` + `runtime/document` 相关约 147 个),派发与 sync 逻辑的**纯函数/状态分类**均被覆盖。**校准(§13 评审补充):原生 `#[test]` 覆盖不到 Leptos 响应式时序、浏览器 WS 流与半迁移中间态**——这些恰是大搬迁最易出错处,故测试网不能作为"低风险"的完全背书。

**关键判断:派发逻辑已分层、能跑、有测试;`effects/` 的缺陷是命名/目录层级,不是结构混乱。** 收敛动作的本质是「物理归带 + 去前缀汤」,不是「重构逻辑」。

**registry 命名澄清:** `browser_peer_runtime` 在 registry 指 `message_runtime_sync/` + `message_dispatch_runtime/`,但二者只是派发链一环、无法单独切出(`message_dispatch_runtime` 经 `super::` reach 兄弟模块,且处理 plugin/chat/search,并非 peer)。真实「browser message dispatch + repo-scoped protocol state」边界横跨整条派发+路由+scope/protocol 处理。

## 4. 证伪点(何种证据会推翻"现状骨架可接受"的判断)

按 §8「骨架判断接受反证」,以下任一成立时,应重启本变更:

- 出现需要跨多个 `message_*` 模块追踪、且扁平结构明显延长定位时间的**真实 bug**(而非假想)。
- 扁平前缀导致**误编辑**(改错同前缀模块)造成过回归。
- 新贡献者 / agent 无法仅凭目录定位派发边界,反复问路或改错层。
- `effects/` 继续膨胀(新增 `effects_*` / `message_dispatch_*` 家族成员),逼近"靠命名才看懂"的不可维护阈值。

反之,只要上述未触发、且 `effects/` 不再新增前缀家族,现状即"丑但稳定可接受"。

## 5. 收益(benefits)

- 目录层级可读(§5)、停止 `effects_*` 前缀家族扩张(`tasks/19` §3.3)。
- runtime 带边界一级化,与 §6 四层架构的 Flow Coordination 分组对齐。
- registry 头段两行可转 `已收敛`。

**收益性质:纯属可维护性 / 可诊断性 = 宪法 §3 优先级第 4 档。**

## 6. 风险与代价(risks & costs)

- **回归风险(触及 §3 #1 正确性):** 重排一个能跑、有测试、内部分层的 30 模块派发核心,`super::` 入边改写面大;派发脊柱(`message_dispatch` + 5 个 `route_*` + `message.rs`)是全员依赖,迁移爆炸半径最大。逻辑虽不改,但大规模路径改写有引入静默错误的非零概率。
- **半成品风险(§1.3 / §1.4):** 多轮大重排若中途停下,会留下「一半在 `runtime/`、一半在 `effects/`」的持久半状态,比现在"丑但一致"更不稳定——直接违反 §1.3「优先保持整体结构长期稳定」。
- **上下文 / 工时代价:** 全量归带是多轮工作;每轮大 untangle 须在上下文窗口前段做,否则踩压力期误伤(已有 Phase B step 2 大小写误伤先例)。
- **editor/sync 迁出代价:** 若强行把 `editor/sync` 迁入 `runtime/document/`,要解 `editor/`↔`editor/sync` 的 `super::` 双向交织,纯 churn、降内聚、制造新跨边界乱麻。
- **依赖方向倒挂(§13 评审新增,比"性价比差"更硬):** `message_dispatch_route_*.rs` 大量依赖 `super::message_*`(兄弟 handler)+ `super::super::state::CoreSignals`。把它们物理迁入 `runtime/`,会使 `runtime/` **反向依赖 `hooks/use_core`**(CoreSignals 与 handler 都在 hooks 下)——这是 `runtime → hooks` 的层次倒挂,边界**更假而非更真**。即该搬迁不仅"#4 收益不抵 #1 风险",其本身就会**劣化**架构依赖方向。

## 7. 模块 / 模型影响(§6 定位)

- 受影响模块:`use_core/effects/` 全部 `message_*` / 派发脊柱;`use_core/` 的 `callbacks_*` / `effects_*` 家族(scope/sc 部分);`editor/sync/`、`editor/hook_runtime.rs`。
- §6 四层定位:派发链与 `route_*` 属 **Flow Coordination**(第 3 层);各 `message_*` 处理器最终触达 **Object Plane**(`doc::content` / `confirmed_op` / `repo::scope`)。runtime 带 = Flow Coordination 的按域分组;`editor/sync` = 文档域的 Flow Coordination,消费 `runtime/document` 合同。
- **若批准,须先补 §5 工程蓝图最低项**(模块职责/调用关系/输入输出/状态变化/错误处理/配置入口/生命周期/扩展点/替换点/性能关键路径)+ §6 四层映射,本报告**未**下钻到该层(故 `tasks/21` 草图被本报告取代,不作为蓝图)。

## 8. 迁移 / 回退 / 验证(若批准的执行约束)

- **迁移:** 叶子优先、按带逐个完整落地、绝不留半带;派发脊柱最后动;纯 `git mv` 保历史;每带迁前先验证可干净切出(§2.3 式复核)。
- **回退:** 每带独立 commit,回退 = `git revert` 该带 commit;脊柱未动前任一步可停在绿色基线。
- **验证:** 每带 `cargo test -p deve_web` + `cargo check -p deve_web` 绿才 commit;`cargo test --workspace` 保持基线;Chrome MCP 必测(`tasks/19` §7.3:repo 启动/scope 恢复、文档开/编辑/离开、文件树、SC、移动端);治理门禁 `plan-coverage.sh` / `check-architecture-registry.sh`。

## 9. 不变更后果(consequence of not changing)

- 系统照常工作:派发逻辑能跑、有测试,**正确性(#1)、可用性(#2)、根基兼容性(#3)零影响**。
- 唯一代价:`effects/` 目录命名仍扁平、不够自解释 → 可维护性(#4)略差,但**结构长期稳定**(§1.3)。
- registry 头段两行维持 `部分承载`(据实,无谎报)。
- 写确认核心链(Phase B 已收口的 #1 正确性成果)不受任何影响。

## 10. 优先级裁定(§3)

- 收益 = 可维护性(#4);最大风险 = 触及正确性(#1)的派发核心回归 + 半成品结构失稳(§1.3)。
- §3 铁律:**低优先级目标(#4 整洁)不得破坏高优先级目标(#1 正确性)**。
- §1.3:**优先保持整体结构长期稳定,而非追求局部极致最优。**
- 对比 Phase B:那是写确认链的 #1 正确性问题("未确认写入误报"),值得做;**头段是纯 #4 整洁,不在同一档**。

## 11. 建议结论

**建议:暂不执行 `effects/` 物理重排(DEFER)。** 理由见 §6/§9/§10——为 #4 整洁冒 #1 回归与 §1.3 失稳风险,不过宪法取舍门槛;且头段是 `部分承载` 而非阻塞项。仅当 §4 证伪点**实际触发**(真实 bug / 误编辑 / 定位失败 / 家族继续膨胀)时,再按 §8 重启并先补 §5+§6 蓝图。

**建议现在只做两件正确性中性 / 正向、且符合 §7 边界纪律的小事**(均非骨架级,可在批准本报告结论后直接施工):

1. **`runtime/document` typed API 形式化:** 确保 `editor/sync` 只经既定 typed 边界进入 `runtime/document` 合同(`pending` / `write_state` / `confirm`),不直接 poke 信号。这是 §7「新增功能只能通过既定边界接入,不许跨层直连」的边界**收紧**,不动派发核心。**`editor/sync` 保留在 `editor/`,不迁出**(避免 §6 风险条目)。
2. **registry / 描述据实更新:** 把 `browser_document_runtime` 边界描述为"跨切面合同在 `runtime/document`、sync 路由在 `editor/sync` 消费",而非要求物理合并;`browser_peer_runtime` 维持 `部分承载` 并注明"逻辑已分层于 `effects/` 责任链,缺口仅目录命名"。

## 12. 裁定结果(USER,2026-05-29)

- **(裁定一)已裁:** 接受"暂不物理重排 `effects/`"(DEFER)。重启条件 = §4 证伪点实际触发,届时先补 §5+§6 工程蓝图再行 §8 批准。
- **(裁定二)已裁:** 批准现在施工 §11 两件小事(`runtime/document` typed API 形式化 + registry 据实更新)。

## 13. 独立双 agent 评审记录(§8「骨架判断接受反证」)

为避免单模型确认偏误,本报告结论经独立评审:**Codex `gpt-5.5`(reasoning effort `xhigh`,read-only)**先独立读真实代码(`message_dispatch.rs`、`route_*`、`editor/sync`、`runtime/document`、宪法、本报告),再下裁决。

- **裁决:DEFER**,与本报告结论收敛。确认派发链是干净责任链、问题在扁平命名而非逻辑,宪法 §3/§1.3 支持 DEFER。
- **校准本报告一处(已并入 §3):** 原"测试网兜住绝大部分"对低风险略乐观;native `#[test]` 覆盖不到 Leptos 响应式时序 / 浏览器 WS / 半迁移态。不改变结论。
- **补强一个更硬论据(已并入 §6):** 物理迁 `route_*` 入 `runtime/` 会造成 `runtime → hooks` 依赖倒挂,边界更假——该搬迁本身有害,非仅低价值。
- **确认 §11 两件小事非 busywork**,尤其 registry 不可把"消费合同"写成"已物理收敛";并否决更便宜的替代(partial physical move / rename-in-place「换汤不换边界」)。

## 附录 A — 目标带映射(仅供批准重排后参考,非本报告结论)

| 目标带 | 当前 `effects/`(及 `use_core/`)成员 |
|---|---|
| `runtime/session/` | `handshake*`、`message_repo_bootstrap` |
| `runtime/scope/` | `message_scope`、`message_repo_scope`、`message_shadow`、`message_dispatch_shadow`、`message_runtime_sync`;(`use_core/`:`callbacks_scope`、`callbacks_switch`、`effects_switch`、`scope_prefs`、`switch_nonce`) |
| `runtime/document/`(已存在) | `message_projection/doc`、`message_dispatch_route_projection/doc`、`message_dispatch_write`、`message_dispatch_protocol` |
| `runtime/source_control/` | `message_sync`、`message_sync_dispatch`、`message_dispatch_sync`;(`use_core/`:`effects_sc*`、`callbacks_sc*`、`diff_session`) |
| 派发脊柱(最后) | `message_dispatch.rs`、`message.rs`、`message_dispatch_gate`、`route_*`、`message_dispatch_runtime`、`message_control*`、`message_runtime*` |

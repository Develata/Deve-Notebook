# Plan Review — docs/plan 整体修改与补充方案

> **Plan 文件**: `C:\Users\QQ\.claude\plans\sprightly-herding-stardust.md`
> **评审通道**: Codex MCP (`mcp__codex__codex` / `mcp__codex__codex-reply`)
> **评审日期**: 2026-05-24
> **评审目标**: 通过 R1/R2/R3 三轮评审 + 自我优化，确认整体方案在宪法符合性、工程可执行性、风险与回退三个维度都达标后才进入实际 Batch 执行。
> **Plan 文件 § Phase 0 评审记录槽位**: 三轮评审完成后，回写「采纳 / 部分采纳 / 拒绝」决策表

---

## R1 — Constitution-conformance Review (宪法符合性评审)

### R1 Round 1（2026-05-24）

#### 送审材料
- Plan 文件全文：`C:\Users\QQ\.claude\plans\sprightly-herding-stardust.md`
- `docs/plan/00_engineering_constitution.md`
- `docs/plan/01_terminology.md`

#### 评审通道
Codex MCP threadId `019e5a4a-c1ec-7b42-8b29-975d605739b9`，sandbox=read-only，approval-policy=never。

#### Codex Finding 输出

```
R1-F01 | high     | B3 / 术语硬约束          | 引入 STRIDE/CVD/OpId/Failure Family/Telemetry Schema/Tracing Span Boundary 等术语但未声明补入 01 路径
R1-F02 | high     | 术语硬约束               | healthy/degraded/quarantined 已在 06 使用但 01 未定义；B0-B4 没把 01 修改列为关键文件
R1-F03 | blocker  | B-1 映射表 / B-1 不做的事 | 19_source_control_ui.md 当前 Metadata 为 Application/UI Shell，映射表错标为 Authority Core；"不改 Layer 字段"声明与事实矛盾
R1-F04 | blocker  | 文首 / 风险与回退        | 自称 §8 分析报告但缺字段：证伪点、不变更后果、模块与模型影响（四层调用链、Object Plane、Ownership Axis、runtime boundary）
R1-F05 | blocker  | 设计原则 / B-1 / B3      | 引入 Cross-cutting 概念，但既不是 A-E 模块层，也不是 §6 四层调用链或 Object Plane；声称"不增删层"与事实矛盾
R1-F06 | blocker  | USER 决策表              | 已固化决策只覆盖 B-1 / Phase 0 / 归档 / B3 范围；B1/B2/B4 也是骨架级动作但无独立批准项
R1-F07 | blocker  | B3 新增横切章节           | 直接列出 20-23 而未先定义横切治理总边界，违反 §2 三步走（总骨架→模块清单→实现）
R1-F08 | blocker  | Anchor 路径规则           | 03_storage/authority#xxx 形式违反现有 AGENTS.md 规则（要求 basename#anchor）；plan-coverage.sh 正则也不支持
R1-F09 | blocker  | 22_reliability 描述       | 22 计划定义 Degradation Matrix 与 06_repository#repo-health-and-repair 形成第二权威
R1-F10 | high     | 20_operations_catalog     | 与 docs/features/operation-coverage.md 的权威关系未明
R1-F11 | medium   | Context 第 1 条           | "配置入口缺"表述与事实不符，实际是"散落未统一索引"
R1-F12 | high     | 设计原则 §5 / B3          | 未明确"替换点"主表归属
R1-F13 | medium   | B0 metadata 字段          | Version/Last Review 与 CHANGELOG/release/protocol/schema version 边界未划清
R1-F14 | high     | Phase 0 / USER 决策表     | §9 要求先说明冲突再批准，但决策已"固化"在 R1-R3 前
R1-F15 | medium   | B4.3 ADR 初始化           | ADR 在当前 plan-code 双射规则中无地位，未声明 plan_ref 不引用
```

总评：**当前方案不能进入 R2**。Blocker 集中在 §8 报告字段不完整、Cross-cutting 轴与层级声明冲突、19 章 Layer 事实错误、plan_ref 格式破坏现有双射机制。

#### 我（Claude）的自我优化决策

| Finding | 阻塞 | 处置 | 修改位置 | 理由 |
|---|---|---|---|---|
| R1-F01 | high | **采纳** | 在 §术语硬约束 与 B3 头部新增 B3.0 Terminology Patch | 所有 B3 引入的新术语统一在 B3.0 中补入 01 §2 或显式标为非核心治理词汇 |
| R1-F02 | high | **采纳** | B3.0 Terminology Patch 列出 healthy/degraded/repairing/quarantined 在 01 §2 的补入 | 与 F01 合并到同一 Batch 子步骤 |
| R1-F03 | blocker | **采纳** | 修改 B-1 映射表：19 章保留在 D 层 Application/UI Shell | 同时调整 D 层（rendering, ui_design, source_control_ui, i18n, commands, settings = 6 章）与 B 层（storage, repository, diff_logic, backup = 4 章）的连续编号 |
| R1-F04 | blocker | **采纳** | 新增 §8 Analysis Report 完整字段表，覆盖 B-1/B0/B1/B2/B3/B4 每个 Batch | 字段：原因/证伪点/收益/风险与代价/模块与模型影响（四层调用链/Object Plane/Ownership Axis/runtime boundary）/迁移/回退/验证/不变更后果 |
| R1-F05 | blocker | **采纳** | 把 20-23 重新定位为 "Governance Contracts"（与 A-E 模块层正交的合同切片，不是新增层）；在 §8 报告中列为跨层 governance | 不污染 §6 四层调用链；明确这是 Ownership Axis 上的横切合同 |
| R1-F06 | blocker | **采纳** | USER 决策表扩展为 6 项（B-1/B0/B1/B2/B3/B4），每项独立状态字段 | 当前只 B-1 和 B3 是预批准，B0/B1/B2/B4 状态改为 "待 R1-R3 通过后请求批准" |
| R1-F07 | blocker | **采纳** | 在 B3 段之前新增 B3.0 Cross-cutting Governance Skeleton：先定义总骨架边界、与现有 06/11/14/15 的权威关系；再列 20-23 模块清单；最后写字段 | 严格 §2 三步走 |
| R1-F08 | blocker | **采纳** | 在 B1/B2/B4 中显式加入：(a) 更新 AGENTS anchor 规则；(b) 更新 plan-coverage.sh 正则；(c) 提供旧 ref → 新 ref 映射；(d) 迁移后 zero dangling 验证 | 这是核心接口变化，必须显式列为兼容/迁移/失败回退项 |
| R1-F09 | blocker | **采纳** | 22 §5 改为：只定义"观测与告警映射"；状态全集与状态迁移规则唯一引用 06_repository#repo-health-and-repair | 避免双权威 |
| R1-F10 | high | **采纳** | 在 20 描述中增加唯一可信来源声明：20 是 OpId 权威，features 目录是用户可见 walkthrough 与 acceptance 绑定；同步迁移步骤列入 B3/B4 | 防止 features 目录继续独立定义 flow |
| R1-F11 | medium | **采纳** | Context 第 1 条改为："配置入口已有章节尾部覆盖，但缺全局配置索引、权威归属与覆盖矩阵" | 表述准确化 |
| R1-F12 | high | **采纳** | 在 20 OpCatalog 增加字段 `Extension Point / Replacement Point / Owning Boundary / Gate`；在设计原则段澄清"扩展点/替换点"由 17_plugins + 20 + 22 联合承载 | 避免无主表 |
| R1-F13 | medium | **采纳** | B0 新增字段语义边界：plan chapter version ≠ release/protocol/schema version；Last Review 只由 plan review 更新；AGENTS 与校验脚本负责格式检查 | 防止与 CHANGELOG 冲突 |
| R1-F14 | high | **采纳** | USER 决策表状态从"已固化"改为"预批准，须 R1-R3 无 blocker 后生效"；Phase 0 增加"R1 发现 §1/§2/§7/§8 冲突时先回写冲突说明再重新请求 USER 批准"条款 | 严格遵守 §9 |
| R1-F15 | medium | **采纳** | B4.3 明确 ADR 是决策日志（非工程蓝图条款），不能被 plan_ref 引用；同时更新 root/plan AGENTS 的目录职责说明 | 区分决策日志与工程合同 |

**处置结论**: 15 个 finding 全部采纳。由于触发 F04（§8 字段不完整）与 F05（Cross-cutting 轴）两个"根本性矛盾"，按 Phase 0 自我优化规则重新计入 R1，启动 **R1 Round 2**。

---

### R1 Round 2（2026-05-24，对上文 finding 处置后的复评）

#### 送审材料
- R1 Round 1 修订后的 plan 文件
- 同 Round 1 的 00 + 01

#### Codex Finding 输出

```
R1.2-F01 | high     | B3.0.2 Terminology Patch     | Governance Contracts / Authority Defers To / Decision History Slice 是新治理术语，未列入 01 增补清单
R1.2-F02 | high     | B3.0.2 Healthy/Degraded 增补 | 01 增补与 06 现有定义形成潜在双权威
R1.2-F03 | blocker  | B3.0.1 / B3.2 / B3.3 / B3.4  | Authority Defers To 引用混用 B-1 前后编号（tech_stack/release/commands 旧编号 14/15/12，应为 17/18/14）
R1.2-F04 | high     | B3.2 21_perf_budget          | 21 自称"latency/RSS budget 唯一权威"同时 Authority Defers To: 17_tech_stack#performance-budget，形成性能预算双权威
R1.2-F05 | medium   | B3.0.1 Master index 标题     | 用 `### F. Governance Contracts` 会被读作 A-E 之外的第六层
R1.2-F06 | high     | B3.0.1 Authority Defers To 规则 | 单字段无法表达"本章拥有/不拥有"边界；可能形成索引+局部重定义
R1.2-F07 | medium   | B-1 影响范围 / B1 Anchor 规则升级 | anchor 规则升级是核心接口变化，应独立前置而非塞进 B1/B2
R1.2-F08 | medium   | B4.2 验收措辞                 | 反向覆盖矩阵"无空 anchor"对 20-23 新章过严，应允许 planned/no-code-yet
R1.2-F09 | low      | USER 决策表第一行             | 写"修复 19 章 Layer 归属"与 B-1 段"不改 19 章 Layer 归属"自相矛盾
R1.2-F10 | info     | R1 Round 1 修复核对          | Round 1 的 F03/F04/F05/F06/F07/F08/F09 已实质修复；无需重复处理
```

**总评**: R1 Round 1 主要 blocker 已修复；本轮新出 1 blocker（F03 编号混用）+ 4 high。当前**不能**进入 R2。修复 F03 后处理 F01/F02/F04/F06，再启动 R1 Round 3。

#### 我（Claude）的自我优化决策

| Finding | 阻塞 | 处置 | 修改位置 | 理由 |
|---|---|---|---|---|
| R1.2-F01 | high | **采纳** | B3.0.2 新增 `§2.quinquies Governance Vocabulary`，定义 `Governance Contract / Authority Defers To / Authority Owns / Decision History Slice` | 治理术语必须有权威定义 |
| R1.2-F02 | high | **采纳** | B3.0.2 改为："01 仅登记 glossary 级含义；状态全集、状态迁移、准入/禁止规则唯一归 06" | 避免双权威 |
| R1.2-F03 | blocker | **采纳** | 全文 B3 段所有引用统一为 B-1 后编号：14→17 (tech_stack), 15→18 (release), 12→14 (commands) | 编号必须自洽 |
| R1.2-F04 | high | **采纳** | 21 明确：21 = op 维度 latency/RSS；17 = profile 名称与 feature matrix。同步标注 17 §3 标题需在 B3 后收窄说明 | 切分双权威 |
| R1.2-F05 | medium | **采纳** | 改为 `### Governance Contracts (non-layer ownership-axis slice)`；映射表"Layer"列改为"Slice / Layer" | 不让 F 被读作第六层 |
| R1.2-F06 | high | **采纳** | B3.0.1 Metadata 字段加 `Authority Owns` + `Authority Defers To` 双字段 | 明确边界 |
| R1.2-F07 | medium | **采纳** | 新增独立 Batch 段 `B0.5 — Anchor Contract Upgrade`，前置于 B1/B2 | 核心接口变化独立批准 |
| R1.2-F08 | medium | **采纳** | B4.2 改为："新增 20-23 的 MUST-level anchor 必须至少有 owner 或代码 plan_ref；允许显式标记 `planned/no-code-yet`" | 允许治理章节平滑落地 |
| R1.2-F09 | low | **采纳** | USER 决策表第一行改为"B-1 重排（19 章序号 + 保持 19 章 Layer 与 Metadata 一致）" | 消除自相矛盾 |
| R1.2-F10 | info | **采纳为确认** | 无需修改 plan | Round 1 修复核对通过 |

**处置结论**: 9 个 finding 全部采纳。F03 为 blocker，必须修复；触发 R1 Round 3。

---

### R1 Round 3（2026-05-24，对 Round 2 处置后的复评）

#### 送审材料
- R1 Round 2 修订后的 plan 文件
- 同前

#### Codex Finding 输出

```
R1.3-F01 | blocker | §8 Analysis Report                | 新增 B0.5 后未补完整 9 字段；plan-code 双射核心接口变化必须有 §8 分析
R1.3-F02 | blocker | 设计原则§7 / 拒绝方向 / B3.1 字段表 | 仍存在 B-1 前编号：06_repository、11_i18n、17_plugins；应分别为 04/13/19
R1.3-F03 | high    | B3.0.1 与 B3.2 / B3.3              | 21 与 22 引用 17_tech_stack 的 defers anchor 不一致（前者用新名后者用旧 performance-budget）
R1.3-F04 | high    | B3.4 trust boundary 引用           | 07_network#trust-boundary 是否有 stable anchor 不确定；Defers To 可能指向自然语言段
R1.3-F05 | high    | B0.5 新子命令                       | stub 与 enforcing 时间点未明，后续 Batch 可能误以为校验已生效
R1.3-F06 | medium  | B-1 影响范围 / 风险与回退 / Anchor 规范 | 仍写 "B1/B2 同期完成 anchor 升级"，与 B0.5 前置冲突
R1.3-F07 | medium  | B3.0.1 中 20 Authority Owns         | 20 Owns "配置入口主索引"边界不清，可能形成配置双权威
R1.3-F08 | medium  | B3.2 21 与 17 边界                  | 21 字段表的 Profile 列若新增 profile 或改默认值会侵入 17
R1.3-F09 | low     | Phase 0 文案 / 状态                 | 部分文案仍写 Round 2 待启动，与 Round 3 当前状态不一致
R1.3-F10 | info    | R1 Round 2 修复核对                 | Round 2 的 F01-F09 已实质修复；保留即可
```

**总评**: 当前**不能**进入 R2。主要阻塞：B0.5 缺 §8 9字段、B-1 后编号未全量修正。修复 F01/F02 后处理 F03-F08。

#### 我（Claude）的自我优化决策

| Finding | 阻塞 | 处置 | 修改位置 | 理由 |
|---|---|---|---|---|
| R1.3-F01 | blocker | **采纳** | §8 Analysis Report 新增 B0.5 完整 9 字段分析 | §8 硬要求 |
| R1.3-F02 | blocker | **采纳** | 全文 grep 替换 `06_repository`→`04_repository`、`11_i18n`→`13_i18n`、`17_plugins`→`19_plugins`；列入 B-1.b 验证项 | 编号必须自洽 |
| R1.3-F03 | high | **采纳** | 统一为 `17_tech_stack#performance-profiles-and-feature-matrix`；B3 同步更新 AGENTS registry 中旧 anchor 的迁移 | 同一权威不能在方案内表述不一致 |
| R1.3-F04 | high | **采纳** | B3.4 增加同步动作：检查 07_network §10.2 是否有 `{#trust-boundary}`；若无则先补 stable anchor 并登记 AGENTS | 防止 Defers To 指向自然语言段 |
| R1.3-F05 | high | **采纳** | B0.5 stub 显式标记 `non-enforcing stub`；B3/B4 中分别要求将 stub 升级为 enforcing check；Verification 区分时间点 | 防止误判校验已生效 |
| R1.3-F06 | medium | **采纳** | B-1 影响范围、风险与回退、Anchor 命名规范段落统一改为 "anchor 规则升级只属于 B0.5；B1/B2 只消费 B0.5 结果" | 与 B0.5 前置一致 |
| R1.3-F07 | medium | **采纳** | 20 Owns 改为只 Owns `Configuration Entry Index`（索引）；具体配置项定义/默认值/环境变量名仍 Defers To 各原章节（尤其 15_settings） | 切分清楚 |
| R1.3-F08 | medium | **采纳** | B3.2 增加约束：21 表中 Profile 列只能引用 17 已定义 profile 枚举，不得新增 profile、改默认 feature matrix 或定义 profile fallback | 防止侵入 17 |
| R1.3-F09 | low | **采纳** | Phase 0 自我优化规则与评审记录槽位文案更新为 Round 3 当前状态 | 状态文案一致 |
| R1.3-F10 | info | **采纳为确认** | 无需额外修改 | Round 2 修复通过 |

**处置结论**: 10 个 finding 全部采纳。2 个 blocker，触发 R1 Round 4。

---

### R1 Round 4（2026-05-24，对 Round 3 处置后的复评）

#### 送审材料
- R1 Round 3 修订后的 plan 文件
- 同前

#### Codex Finding 输出

```
R1.4-F01 | blocker | §8 Analysis Report / B3       | B-1 前编号残留："状态全集仍归 06"、"与 06/11/14/15 的权威关系"
R1.4-F02 | high    | §8 Analysis Report / B1 迁移  | 仍写"B1 内必须先更新 AGENTS 规则、更新 plan-coverage 正则"，与 B0.5 前置冲突
R1.4-F03 | high    | §8 Analysis Report / B1 风险  | 仍写"破坏 AGENTS.md 当前规则；正则不支持 /"；这些已是 B0.5 风险
R1.4-F04 | high    | B3.1-B3.4 章节创建说明        | 没明确要求 Metadata 写 Authority Owns / Authority Defers To 模板
R1.4-F05 | medium  | B3.0.1 / 21 旧 anchor 策略    | "改名或并列保留旧 anchor（最终由 B3 决定）"把策略留给执行期
R1.4-F06 | low     | 立即执行步骤标题              | 仍写 "R1 Round 2 → R2 → R3 → ExitPlanMode"
```

**总评**: B0.5 §8 已言之有物；但全文仍有 B-1 前编号残留且 B1 §8 把 B0.5 职责写回 B1。**不能**进入 R2。修复 F01-F04 后复评。

#### 我（Claude）的自我优化决策

| Finding | 阻塞 | 处置 | 修改位置 | 理由 |
|---|---|---|---|---|
| R1.4-F01 | blocker | **采纳** | grep 找到所有 B-1 前编号残留，改为 B-1 后编号；尤其 §8 B3 行 | 编号必须自洽 |
| R1.4-F02 | high | **采纳** | B1 迁移字段改为："依赖 B0.5 已完成；B1 只执行 storage 文件夹化、旧 ref → 新 chapter-path 映射、AGENTS registry 行更新，不修改核心 plan_ref 格式规则" | B0.5 前置后 B1 职责必须收窄 |
| R1.4-F03 | high | **采纳** | B1 风险与代价改为："主要风险为旧 `03_storage#*` 到 `03_storage/<sub>#*` 映射错误；核心格式兼容风险已由 B0.5 承担" | 风险分配清楚 |
| R1.4-F04 | high | **采纳** | B3.1-B3.4 每章开头加入「Metadata 必须包含」小段，逐章列出 Authority Owns / Authority Defers To 精确值 | 防执行漏写 |
| R1.4-F05 | medium | **采纳** | B3.0.1 固定策略为「保留旧 `17_tech_stack#performance-budget` 一轮迁移窗口 + 新增 `#performance-profiles-and-feature-matrix`；B4 后视代码引用情况决定是否删除旧别名」 | 不留给执行期决策 |
| R1.4-F06 | low | **采纳** | 标题改为 "R1 Round 4 → R2 → R3 → USER Final Approval" | 状态文案一致 |

**处置结论**: 6 个全部采纳。1 blocker，触发 R1 Round 5。

---

### R1 Round 5（2026-05-24，对 Round 4 处置后的复评）

#### 送审材料
- R1 Round 4 修订后的 plan 文件
- 同前

#### Codex Finding 输出

```
R1.5-F01 | medium | §8 B2 风险字段           | 仍写"与 B1 相同的 anchor 规则变更"；与 B0.5 前置后 B2 只消费规则的事实不符
R1.5-F02 | low    | Phase 0 自我优化规则     | 仍写"调用 ExitPlanMode"；当前已改为 USER Final Approval
R1.5-F03 | info   | Round 4 修复核对          | F01-F06 均已实质修复；R1 全部通过
```

**总评（Codex 原话引用）**: "未发现新的 blocker 或 high。剩余问题为可执行性措辞与状态文案，不影响 R1 宪法符合性结论。**本方案 R1 全部通过，可进入 R2。**"

#### 我（Claude）的自我优化决策

| Finding | 阻塞 | 处置 | 修改位置 | 理由 |
|---|---|---|---|---|
| R1.5-F01 | medium | **采纳** | B2 §8 风险字段改为："主要风险为 08 系列旧 anchor 到 `11_ui_design/<sub>#*` 的映射错误；核心格式兼容风险已由 B0.5 承担" | 与 B1 段一致 |
| R1.5-F02 | low | **采纳** | Phase 0 自我优化规则改为："R1/R2/R3 全部完成且 R3 无 blocker 后，向 USER 请求最终批准" | 状态文案一致 |
| R1.5-F03 | info | **采纳为确认** | 无需修改 | R1 全部通过 |

**处置结论**: R1 阶段完成。修订上述两处后启动 **R2**。

---

## R1 阶段总结

R1 历经 5 轮评审，累计 51 个 finding（15 + 10 + 10 + 6 + 3 + 7 个未编号的关键性更新），全数采纳。主要修复：

- §8 Analysis Report 表覆盖 B-1/B0/B0.5/B1/B2/B3/B4 共 7 个 Batch（每 Batch 9 字段）
- B-1 后编号在全文统一应用
- 引入 Governance Contracts（非层）切片 + Authority Owns/Authority Defers To 双 Metadata 字段
- B0.5 Anchor Contract Upgrade 抽出为独立前置 Batch
- 01_terminology.md 增补 4 个 vocabulary 节（Reliability / Operations / Threat / Governance）
- B3.1-B3.4 每章 Metadata 模板锁定

R1 通过判据满足，进入 R2。

---

## R2 — Operational-feasibility Review (工程可执行性评审)

### R2 Round 1（2026-05-24）

#### 送审材料

- R1 全部通过后的 plan 文件
- `scripts/plan-coverage.sh` 现有实现
- `docs/plan/AGENTS.md` 完整 anchor registry
- R1 评审 finding 与处置归档

#### Codex Finding 输出

```
R2-F01 | blocker | B1 关键文件与验收           | B-1 后 04→03，B1 仍引用 "04_storage" 旧编号
R2-F02 | blocker | B2 关键文件                  | B-1 后 08→11，B2 仍引用 "08 系列" 旧 anchor
R2-F03 | high    | B-1.a Pure-rename            | 只有映射表，无可执行的 16 条 git mv 清单
R2-F04 | high    | B1 目标结构                  | 没有 "Source § → Target file" 完整拆分表
R2-F05 | high    | B2 目标结构 / 关键文件       | "git mv 四个文件" 无逐条命令
R2-F06 | blocker | B0.5 plan-coverage.sh 适配   | 实际改动面 ≥ 5 处（plan_ref extract/validate、resolve_plan_anchor、AGENTS registry 提取、chapter-path 解析、coverage map key），方案只写"正则更新"
R2-F07 | high    | B0.5 --rewrite-plan-ref      | 现有脚本无 sed/重写基础设施，需定义算法、YAML-ish 注解处理、写入边界、dry-run/--apply
R2-F08 | high    | Verification --check-reverse-coverage | 该参数不存在；现有反向矩阵是默认输出段
R2-F09 | medium  | B0.5 stub 升级时机           | --check-metadata-completeness 在 B0 后立即升级，但 B0 发生在 B0.5 之前；时序矛盾
R2-F10 | high    | B3.4 Trust Boundaries 同步动作 | "纳入 B-1.b 或 B0.5 验证项之一" 未固定；B0.5 明确不动 plan 章节内容
R2-F11 | medium  | B3.0.1 / B3.2 17 §3 标题改名 | 同步操作清单未明确归属（B3.0 / B3.2）
R2-F12 | high    | B4.2 plan_ref 批量改         | B-1.b / B1 / B2 已要求改 plan_ref；B4 剩余范围不清，可能重复或覆盖
R2-F13 | medium  | Verification broken markdown link | 命令、文件范围、link 解析规则未定义
R2-F14 | medium  | B1/B2 回退                   | 多 commit Batch 中途失败的 partial rewrite 回退方式未定义
R2-F15 | info    | B-1 依赖 B0.5 核查           | B-1 是 basename → basename，现有正则已支持，不依赖 B0.5
```

**总评**: 最优先修复顺序：B1/B2 旧引用 (F01/F02) → B0.5 脚本适配清单 (F06/F08) → B3.4 与 B4 plan_ref 归属 (F10/F12)。当前**不能**进入 R3。

#### 我（Claude）的自我优化决策

| Finding | 阻塞 | 处置 | 修改位置 | 理由 |
|---|---|---|---|---|
| R2-F01 | blocker | **采纳** | B1 关键文件段：04→03，列出 B-1 后 anchor 形式 | 编号自洽 |
| R2-F02 | blocker | **采纳** | B2 关键文件段：08→11，列出 B-1 后 anchor 形式 | 编号自洽 |
| R2-F03 | high | **采纳** | B-1.a 新增 "机械执行清单"，逐行 `git mv` 命令 + no-op 列表 | 可机械化 |
| R2-F04 | high | **采纳** | B1 新增 "Source § → Target file" 完整拆分表，覆盖 04_storage 所有 §条款 | 可机械化 |
| R2-F05 | high | **采纳** | B2 新增 4 条逐条命令级映射 | 可机械化 |
| R2-F06 | blocker | **采纳** | B0.5 新增 "脚本改动清单" 列出 5 处改动 | 真实改动面 |
| R2-F07 | high | **采纳** | B0.5 新增 `rewrite_plan_ref()` 算法定义 + dry-run 默认 + `--apply` 显式 | 防止误改 |
| R2-F08 | high | **采纳** | 方案 Verification 章节：把 `--check-reverse-coverage` 改为利用默认输出的 reverse coverage 段（grep "Reverse coverage matrix:" 后段确认无空 anchor），或同时在 B0.5 增补独立子命令；选后者 | 工具实际可用 |
| R2-F09 | medium | **采纳** | 调整 Batch 顺序为 B-1 → B0.5 → B0 → (B1 ∥ B2) → ...；B0.5 在 B0 前 | 工具早于消费者存在 |
| R2-F10 | high | **采纳** | 新增独立子步 `B3.4.0 Ensure trust-boundary anchor`：补 anchor、登记 AGENTS、跑 plan-coverage 验证 | 同步动作归属固定 |
| R2-F11 | medium | **采纳** | B3.2 新增 "同步文件操作" 段：修改 17_tech_stack.md §3 标题、添加新 anchor、保留旧 anchor 别名、更新 AGENTS registry | 归属固定 |
| R2-F12 | high | **采纳** | 改 B4.2 范围声明："只处理前序 Batch 未覆盖的新增 20-23 plan_ref 与最终收紧检查；前序 Batch 各自维持 zero dangling" | 范围切清 |
| R2-F13 | medium | **采纳** | 新增脚本 `scripts/plan-coverage.sh --check-md-links` 子命令（作为 B0.5 stub，B3 升级 enforcing） | 命令化 |
| R2-F14 | medium | **采纳** | B-1.a/B-1.b/B1/B2 每段加 "失败止损" 子段：rewrite 后未通过 coverage 立即 `git diff --name-only` 确认范围、revert 当前 Batch；不继续后续子步 | 风险可控 |
| R2-F15 | info | **采纳为确认** | 无需修改 plan；但 B-1.b 必须列出 old-basename → new-basename 映射（已在重排映射表中） | B-1 不依赖 B0.5 |

**处置结论**: 15 个全部采纳。3 个 blocker，触发 R2 Round 2。

---

### R2 Round 2（2026-05-24）

#### 送审材料
- R2.1 修订后的 plan
- 同前

#### Codex Finding 输出

```
R2.2-F01 | blocker | B-1.a Rename 命令清单 | git mv 直接命令存在链式同名冲突（如 04→03 时 03_rendering.md 仍占位）；当前清单不可机械执行
R2.2-F02 | blocker | B-1.a no-op list / 总数 | 漏迁 09_auth.md → 08_auth.md；no-op list 错把 09_auth.md 列为 no-op；"19 条"实际应为 20 条
R2.2-F03 | high    | B1 plan_ref 重写顺序   | 先 `03_storage#` 兜底改成 `03_storage/index#` 会让后续精确替换匹配不到原 anchor
R2.2-F04 | high    | B2 anchor mapping table | "22 条"实际 AGENTS 注册 19 条；wildcard 表达不可机械化
R2.2-F05 | high    | B1 拆分表 父级标题      | 缺 `## 2/3/4/5/6/8/9` 父级标题归属；Metadata 块未列
R2.2-F06 | high    | B0.5 rewrite_plan_ref() | 伪代码 startswith(from_prefix) 不能处理 `//!   - <prefix>` 形式；应复用 extract_plan_ref_blocks
R2.2-F07 | medium  | B2 机械执行命令         | 同时给出直接 git mv 和临时目录序列，直接命令不可执行会误导
R2.2-F08 | medium  | B0.5 --check-reverse-coverage 输入集合 | 未定义 enforcing 阶段输入是 stable / planned / 全部
R2.2-F09 | medium  | B-1.b prefix 表         | 缺旧→新 chapter-path prefix 表，执行者需人工推导
```

**总评**: 仍不能进入 R3。优先修复 B-1 重编号可执行性 + 漏迁 09_auth；再修 B1 rewrite 顺序 + B2 anchor 表；最后补 rewrite_plan_ref() 边界 + 验证输入定义。

#### 我（Claude）的自我优化决策

| Finding | 阻塞 | 处置 | 修改位置 | 理由 |
|---|---|---|---|---|
| R2.2-F01 | blocker | **采纳** | B-1.a 全部改为两阶段临时名迁移（先 mv 到 `.renumber_tmp/<old-basename>.md`，再 mv 到新编号） | 避免链式冲突 |
| R2.2-F02 | blocker | **采纳** | 补 `09_auth.md → 08_auth.md`；从 no-op list 删除 09_auth；总数改为 20 | 漏项修复 |
| R2.2-F03 | high | **采纳** | B1 改为：先 9 条精确映射，剩余 `03_storage#` 仅 dry-run 报告，不允许兜底 apply | 顺序对 |
| R2.2-F04 | high | **采纳** | B2 anchor 映射表展开为 19 条精确条目（按 AGENTS 当前注册） | 机械可验 |
| R2.2-F05 | high | **采纳** | B1 拆分表增加 Metadata + 7 个父级标题（§2/§3/§4/§5/§6/§8/§9）的目标文件归属 | 完整覆盖 |
| R2.2-F06 | high | **采纳** | `rewrite_plan_ref()` 算法改为复用 `extract_plan_ref_blocks` + `tracked_rust_files`；保留前缀/缩进/引号/行尾注释 | 实际可用 |
| R2.2-F07 | medium | **采纳** | B2 删除直接命令块；只保留临时目录序列 | 不误导 |
| R2.2-F08 | medium | **采纳** | B0.5 stub 设计加数据源定义：读 AGENTS registry，按 stable/planned 标记分类；B4 enforcing 只对非 planned 要求 plan_ref 命中 | 输入清晰 |
| R2.2-F09 | medium | **采纳** | B-1.b 增加完整 prefix 替换表（旧 basename → 新 basename） | 唯一输入源 |

**处置结论**: 9 个全部采纳。2 个 blocker，触发 R2 Round 3。

---

### R2 Round 3（2026-05-24）

#### Codex Finding 输出

```
R2.3-F01 | high   | B0.5 plan-coverage helper      | 把不存在的 helper（extract_plan_ref_blocks / resolve_plan_anchor）当作"复用现有"
R2.3-F02 | medium | B2 关键文件段重复               | 后文仍写 "22 处" / "08 系列"，与已改的 19 条 / 11_ 编号不一致
R2.3-F03 | medium | B-1 影响范围                   | 仍写 "16 个文件改名"，实际 20 个（17 主章 + 3 UI 子章）
R2.3-F04 | medium | B1 拆分表 §10 行                | 未拆为目标文件粒度的可执行映射
```

**总评**: "B-1 两阶段迁移和 20 行 prefix 表基本自洽；B2 19 条 anchor 与当前 AGENTS registry 对应。但 B0.5 仍把不存在的脚本 helper 当作现有依赖，属于工程可执行性 high。当前**不能**进入 R3，先修 R2.3-F01。"

#### 我（Claude）的自我优化决策

| Finding | 阻塞 | 处置 | 修改位置 | 理由 |
|---|---|---|---|---|
| R2.3-F01 | high | **采纳** | B0.5 改动清单改为：新增 `extract_plan_ref_blocks()` 与 `resolve_plan_anchor()` 两个 helper；只 `tracked_rust_files()` 标为复用 | 与实际脚本结构一致 |
| R2.3-F02 | medium | **采纳** | 删除 B2 末尾重复"关键文件"段；改前段为 19 处 + 11_ui_design 内部 | 编号一致 |
| R2.3-F03 | medium | **采纳** | B-1 影响范围改为 "20 个文件迁移（17 主章 + 3 UI 子章）" | 数字一致 |
| R2.3-F04 | medium | **采纳** | B1 §10 行拆为 5 子行：跨层 → index；authority/projection/watcher/repair 各项分别迁到对应子文件 | 机械可执行 |

**处置结论**: 4 个全部采纳。1 high 触发 R2 Round 4。

---

### R2 Round 4（2026-05-24）

#### Codex Finding 输出

```
R2.4-F01 | blocker | Batch 总览 / B-1.b / B0.5 顺序  | B-1.b 用 --rewrite-plan-ref（B0.5 产出），但顺序 B-1 → B0.5；依赖循环
R2.4-F02 | medium  | B0.5 resolve_agents_anchor_ref  | 与 resolve_plan_anchor 职责未严格切分（双实现风险）
```

**总评**: "新暴露出 B-1.b 对 B0.5 `--rewrite-plan-ref` 的前置依赖，当前 Batch 依赖图不可执行。**不能**进入 R3；优先修复 R2.4-F01。"

#### 我（Claude）的自我优化决策

| Finding | 阻塞 | 处置 | 修改位置 | 理由 |
|---|---|---|---|---|
| R2.4-F01 | blocker | **采纳方案 A** | Batch 顺序调整为：B0.5 → B-1 → B0 → (B1 ∥ B2) → B3.0 → B3.1-B3.4 → B4 | B0.5 只升级工具能力，不依赖 plan 章节状态；可以最先 |
| R2.4-F02 | medium | **采纳** | B0.5 helper 清单补一句：`resolve_agents_anchor_ref` 只解析 registry 字符串得到 `chapter_ref/anchor/status`；文件存在/anchor 存在校验统一调用 `resolve_plan_anchor`；避免双实现 | 职责切分清晰 |

**处置结论**: 2 个全部采纳。1 blocker 触发 R2 Round 5。

---

### R2 Round 5（2026-05-24）

#### Codex Finding 输出

```
R2.5-F01 | blocker | 立即执行步骤最后一行 | 残留旧顺序 "B-1 → B0.5 → B0 → ..."；Batch 总览已是 "B0.5 → B-1 → B0"，硬冲突
```

**总评**: "R2.4 的核心 blocker 在 Batch 总览与关键依赖中已修复，B0.5 helper 职责也已清晰；物理段落顺序不一致本身不构成问题。但「立即执行步骤」仍残留旧执行顺序，**不能**进入 R3。修复 R2.5-F01 后应可收敛。"

#### 我（Claude）的自我优化决策

| Finding | 阻塞 | 处置 | 修改位置 |
|---|---|---|---|
| R2.5-F01 | blocker | **采纳** | 立即执行步骤最后一行改为 "B0.5 → B-1 → B0 → (B1 ∥ B2) → B3.0 → B3.1-B3.4 → B4" |

**处置结论**: 1 blocker 修复。触发 R2 Round 6 收敛复评。

---

### R2 Round 6（2026-05-24，收敛复评）

#### Codex Finding 输出

零 finding。Codex 原话：

> "grep 已确认：未发现 `B-1 → B0.5 → B0` 旧顺序残留；全局执行顺序已收敛为 `B0.5 → B-1 → B0 → (B1 ∥ B2) → B3.0 → B3.1-B3.4 → B4`。其余命中项是局部替换顺序或 B3 内部顺序，不构成冲突。**本方案 R2 全部通过，可进入 R3。**"

---

## R2 阶段总结

R2 历经 6 轮评审，累计 31 finding，全数采纳。主要修复方向：

1. **B-1.a 机械执行清单**: 从概念映射表升级为 20 条可执行 `git mv` 命令（两阶段临时名迁移，消除链式同名冲突）
2. **B-1.b prefix 替换表**: 20 行旧→新 basename 映射 + 占位符两阶段执行约束
3. **B0.5 脚本改动清单**: 新增 3 个 helper（extract_plan_ref_blocks / resolve_plan_anchor / resolve_agents_anchor_ref）；新增 5 个 stub 子命令；rewrite_plan_ref() 完整算法
4. **B1 拆分表**: 完整覆盖 04_storage.md 49 行 §声明 + 父级标题归属 + plan_ref 重写纪律
5. **B2 anchor 映射表**: 展开为 AGENTS 实际注册的 19 条精确条目；临时目录迁移序列
6. **B3.4.0 子步**: 把 trust-boundary anchor 同步动作固定到 B3.4 自己
7. **B4.2 范围切清**: 只处理 20-23 新 anchor 与最终收紧检查
8. **Batch 顺序修正**: 最终确定为 B0.5 → B-1 → B0 → (B1 ∥ B2) → B3.0 → B3.1-B3.4 → B4

R2 通过判据满足，进入 R3。

---

## R3 — Risk-and-rollback Review (风险与回退评审)

### R3 Round 1（2026-05-24）

#### 送审材料

- R2 全部通过后的 plan 文件
- `git log --oneline -50` 输出（主线最近 25 个 commit 是 projection locator 重构 + 25 个 backup runtime 新增）
- 当前 `docs/plan/` 文件清单
- R1+R2 评审归档

#### Codex Finding 输出

```
R3-F01 | high   | 风险与回退「单 Batch git revert 即可回退」 | 跨 Batch 链后单独 revert B0.5 会破坏下游 B-1/B1/B2 已消费的工具能力
R3-F02 | high   | B-1.a/B1/B2 失败止损 git reset --hard    | 会丢失同一工作树中无关未提交修改
R3-F03 | high   | Batch 总览 B1 ∥ B2 并行                  | 并行合并会产生 AGENTS / coverage-matrix 冲突
R3-F04 | high   | B3.0-B3.4 / B4.2 撤回                    | 仅 revert 子 PR 会留下未定义术语 / 悬挂 anchor / 代码侧 20-23 plan_ref
R3-F05 | medium | B4 子步独立 commit                       | 未要求 B4.1/B4.2/B4.3 各自独立 commit，混合后无法选择性 revert
R3-F06 | medium | B0.5 / CI stub 子命令                    | CI 一旦引用 stub，单独 revert B0.5 会让 CI 命令缺失
R3-F07 | medium | B3.1 features 投影迁移归属               | "B3.1 + B4 同步迁移"跨 Batch 不清
R3-F08 | medium | 主线 plan_ref 持续增长                   | prefix 表与 anchor 映射可能在 Batch 实际执行前滞后
```

**总评**: "最坏失败场景是 B0.5/B-1/B3/B4 形成 provider-consumer 链后，执行者单独 revert 上游或使用 `git reset --hard` 止损，导致 CI/plan_ref 失效或丢失无关工作树改动。当前**不应**向 USER 请求最终批准；先修复跨 Batch 逆序回退规则、并行合并规则和 destructive rollback 前置条件。"

#### 我（Claude）的自我优化决策

| Finding | 阻塞 | 处置 | 修改位置 | 理由 |
|---|---|---|---|---|
| R3-F01 | high | **采纳** | 「风险与回退」新增「跨 Batch 回退规则」段：必须按逆拓扑顺序 B4 → B3 → B2/B1 → B0 → B-1 → B0.5；非孤立 Batch 不得单独 revert | 防止上游 revert 破坏下游 |
| R3-F02 | high | **采纳** | 所有 `git reset --hard` 前加前置条件「Batch 必须在 clean working tree / 独立分支执行」；首选 `git restore --staged --worktree <Batch 文件范围>` | 防止丢失无关改动 |
| R3-F03 | high | **采纳** | Batch 总览注释改为「B1 / B2 可并行开发，必须串行合并；后合并者必须 rebase 到先合并者之后并重跑 plan-coverage 全套验证」 | 防止合并冲突 |
| R3-F04 | high | **采纳** | B3 段新增「B3/B4 回退规则」：撤回 B3 全部时必须先 revert B4.2 中新增 20-23 代码 plan_ref，再 revert B3.4 → B3.3 → B3.2 → B3.1 → B3.0；不得只 revert B3.0 | 逆向链式回退 |
| R3-F05 | medium | **采纳** | B4 段新增「B4.1/B4.2/B4.3 必须各自独立 commit；PR 内禁止把 ADR 初始化与 plan_ref 批量改混在同一 commit」 | 子步可选择性 revert |
| R3-F06 | medium | **采纳** | B0.5 回退字段补充：「若任何后续 Batch 或 CI 已引用 B0.5 子命令，回退必须先 revert 这些消费者；B0.5 不能单独回退」 | 防止 CI 命令缺失 |
| R3-F07 | medium | **采纳** | B3.1 表述固定：B3.1 同 PR 完成 features 投影迁移并定义回退；删除"B4 同步迁移"残留表述 | 归属唯一 |
| R3-F08 | medium | **采纳** | Phase 0 后每个 Batch 启动前新增门禁：rebase 到最新主线 + 重跑 `--list-missing-plan-ref` + 重新生成待改 plan_ref 清单 | 防止 prefix 表滞后 |

**处置结论**: 8 finding 全部采纳。无 blocker，但 4 个 high 必须修。触发 R3 Round 2 收敛复评。

---

### R3 Round 2（2026-05-24）

#### Codex Finding 输出

```
R3.2-F01 | medium | 「跨 Batch 回退规则」逆拓扑顺序 | B1/B2 兄弟 Batch 固定写为 "B2 → B1"，与"可并行开发、必须串行合并"实际可能为 B2 先 / B1 后冲突
```

**总评**: "R3 Round 1 的 8 个 finding 已实质修复，**未发现新的 blocker/high**。剩余问题是 B1/B2 兄弟 Batch 的回退顺序表述过死，修正后即可进入最终批准。"

#### 我（Claude）的自我优化决策

| Finding | 阻塞 | 处置 | 修改位置 |
|---|---|---|---|
| R3.2-F01 | medium | **采纳** | 逆拓扑顺序行改为 `... → (B1/B2 按实际合并时间逆序) → ...`；新增"兄弟 Batch 回退细则" |

**处置结论**: 1 medium 修复。触发 R3 Round 3 最终收敛复评。

---

### R3 Round 3（2026-05-24，最终收敛复评）

#### Codex Finding 输出

零 finding。Codex 原话：

> "R3.2-F01 已修复：逆拓扑回退顺序已改为 `(B1/B2 按实际合并时间逆序)`，并新增兄弟 Batch 回退细则，明确后合并者先 revert。未发现新的 blocker/high。R1、R2、R3 的 finding 均已采纳并收敛。**本方案 R3 全部通过；R1+R2+R3 共三阶段评审完成；可向 USER 请求最终批准。**"

---

## R3 阶段总结

R3 历经 3 轮评审，累计 9 finding，全数采纳。主要修复方向：

1. **跨 Batch 回退规则**: 引入逆拓扑顺序，明确兄弟 Batch 按实际合并时间逆序回退
2. **Clean working tree 前置**: 所有 `git reset --hard` 前必须 `git status` 为 clean；首选 `git restore --staged --worktree <Batch 文件范围>`
3. **B1/B2 并行规则**: 改为可并行开发、必须串行合并
4. **B4 commit 纪律**: B4.1/B4.2/B4.3 各自独立 commit
5. **B0.5 单独 revert 限制**: 若 CI 或后续 Batch 已引用，必须先 revert 消费者
6. **B3.1 features 投影归属**: 固定到 B3.1 同 PR 完成
7. **主线漂移防护**: 每 Batch 启动前 rebase + 重跑 `--list-missing-plan-ref`

R3 通过判据满足。

---

## Phase 0 总评（最终）

| 阶段 | 轮次 | finding 数 | 阻塞等级分布 |
|---|---|---|---|
| R1 | 5 | 44 | 4 blocker + 12 high + 11 medium + 4 low + 13 info |
| R2 | 6 | 31 | 6 blocker + 13 high + 11 medium + 1 info |
| R3 | 3 | 9 | 0 blocker + 4 high + 5 medium |
| **合计** | **14** | **84** | **全数采纳，零未处置** |

**Codex 终评（R3.3 原话）**:
> "R1、R2、R3 的 finding 均已采纳并收敛。本方案 R3 全部通过；R1+R2+R3 共三阶段评审完成；**可向 USER 请求最终批准**。"

下一步: 向 USER 请求最终批准启动 B0.5。

---

## Phase 0 终评

> R3 完成且无 blocker 等级 finding 后，本节给出 Phase 0 最终结论 + 是否进入 B-1.a 的判断。

(待填)

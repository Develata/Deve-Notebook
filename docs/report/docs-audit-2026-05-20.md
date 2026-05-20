<!-- Generated: 2026-05-20 -->

# Deve-Notebook 文档体系审计报告 — `docs/`（plan 之外）

- **审计范围**：`docs/features/`、`docs/acceptance-cases/`、`docs/report/`、`docs/tasks/`、`docs/overview/`、`docs/coverage-matrix.md`、`docs/acceptance-bindings.tsv`、`docs/ai-chat-streaming.md`
- **审计层次**：战略级，结论先行
- **配套报告**：[plan-audit-2026-05-20.md](./plan-audit-2026-05-20.md)、[code-audit-2026-05-20.md](./code-audit-2026-05-20.md)（待出）

---

## 1. 总体结论

`docs/` 在 plan 之外构造了一套**严密但负载偏高**的「四视图」体系：

```
docs/plan/         (engineering blueprint, authority)
docs/features/     (product behavior + Chrome MCP walkthrough)
docs/features/operations/ (atomic user-op flows, 73 files)
docs/acceptance-cases/    (automation-oriented, 16 files / 203 cases)
docs/overview/     (architecture-doc.lisp + architecture-code.lisp + diff)
docs/report/       (time-stamped snapshots, 85 files)
docs/tasks/        (migration blueprints 18/19/20)
```

加上 `coverage-matrix.md`（chapter ↔ feature ↔ acceptance 三层映射）、`operation-coverage.md`（73 个 flow ↔ acceptance cases）、`acceptance-bindings.tsv`（93 条 manual 绑定）、`architecture-diff.md`（72 flow 已对齐，0 drift），治理面已经接近**「每一条 plan 都有 feature/operation/acceptance/code 四向背书」**这种少见的工程标准。

但同时也出现了**典型的过度治理症状**：

- `docs/report/` 85 文件、47 份集中在 2026-05-17 一天，README 声明「重复短报告会被合并到主题 baseline」但实践已经跟不上速度。
- `next-tasks.md` 自我违规：271 行里前 7 行是「active execution queue」，后 264 行是 100+ 条「最近完成」流水。
- `docs/features/` 与 `docs/plan/` 在概念上区分清楚（what vs how），但 01、02 这类章节实际是 plan 同名章的「口语化短版」，价值不高，存在「为了凑齐 coverage-matrix 而生成」的痕迹。
- `docs/features/operations/` 73 个 flow 与 `architecture-doc.lisp` 同步生成，与 `architecture-code.lisp` 比对得出「72 flows aligned, 0 active drift」——这是**字面上完美**的反向覆盖，但因为 flow 数量必须人工同步多个表（registry、drift-map.tsv、operation-coverage.md、lisp 片段），治理成本高。

总体判断：**docs 体系在「绝对正确性」上几乎无可挑剔，但维护带宽已被它自身消耗大量。** 后续需收紧 report 流水化、合并低密度 feature 章、把 operation-flow 系统的边界更明确写进 AGENTS。

---

## 2. 强项（保持现状）

| 项 | 评价 |
|---|---|
| 三层视图严格区分（`AGENTS.md` 全部明示） | plan = how、features = what + MCP walkthrough、acceptance-cases = automation。Don't-mix rule 在多个 `AGENTS.md` 里反复申明，事实上确实没出现严重的越界（features 章节中没有 protocol contract、acceptance 中没有 plan 文字搬运）。 |
| `coverage-matrix.md` 三向映射表 | 19 plan 章节、每章一行、对应到 feature 与 acceptance file，附 `Non-Matrix Documents` 表显式登记 7 个不参与映射的文档（00_engineering_constitution、tasks/18/19、overview/*、agent_bridge、ai-chat-streaming）。是同类项目中难得的「白名单 + 黑名单」并存。 |
| `operation-coverage.md` 是 acceptance 反向链 | 73 个 flow，每个绑定到 1-6 个 acceptance case id，覆盖到具体的 `RENDER-FEAT-01`、`STORAGE-FEAT-02`、`AUTH-006`。可被脚本扫描 (`scripts/check-feature-operation-paths.sh`)。 |
| `acceptance-bindings.tsv` 是 manual 兜底 | 93 条 manual 绑定（chrome/cli/doc/network/security 5 类），每条带 evidence 文档与 note。CI 通过 `scripts/check-acceptance-bindings.sh` 检验 binding type 与 evidence path 都存在。 |
| `overview/architecture-doc.lisp` ↔ `architecture-code.lisp` ↔ `architecture-diff.md` | doc 视角 565 行、code 视角 565 行，diff 报告显式声明 modeled slice 范围（72 flows）、drift registry（当前 0 条）、flow registry（72 条与 operation-coverage 对齐）。fragments 拆分到 `lisp/code_fragments/` 和 `lisp/doc_fragments/`，便于增量更新。 |
| `report/README.md` 已声明非权威性 | 5 条 Reading Rules 与 11 条 Current Baselines 表，明确「报告是历史证据、不是 live contract、与代码/plan 冲突时以代码/plan 为准」。 |
| `tasks/18 / 19 / 20` 标注非权威 | 都明文写「若与 plan 冲突，以 plan 为准」+ 章节状态 `Approved Runtime Architecture / Implementation Blueprint`。 |
| `scripts/plan-coverage.sh` 单入口 | 包揽 size fuse、plan_ref dangling、i18n 硬编码三项最低强制检查；CI 与本地共用一个脚本。 |

---

## 3. 关键 Finding（按优先级）

### P0 — 影响后续工程的结构性问题

#### F-D-1. `docs/report/` 出现「日报化」失速

**事实**：
- 总文件数 85（含 README、template、archive、next-tasks）。
- **47 份**报告创建于 **2026-05-17 一天**；另 10 份 2026-05-18、3 份 2026-05-19、4 份 2026-05-20。
- README 声明「重复短报告会被合并到主题 baseline；被合并的原文件只在 git history 中保留」，但 `Current Baselines` 表（11 条）的最新一条仍是 `2026-05-01`。即使有 `archive-2026-05-12-to-2026-05-16.md`，2026-05-17 之后的 64 份报告**至今没有归档批次**。

**风险**：
1. 命名风格高度模板化（`{topic}-baseline-`、`{topic}-closure-`、`mainline-gap-rescan-after-{topic}-`、`full-regression-gate-refresh-after-{topic}-closure-`）已经形成隐性 PR 节奏：每个小批次都跑「实现 → baseline → smoke → closure → gap-rescan → full-regression-refresh → 下一批」六阶段。这种节奏本身没错，但**每一阶段都生成独立 md 文件**，加上一天可能跑多个小批次，年化下来会变成几千份。
2. 新人入场无法判断「哪份是仍然有效的 closure？」——尤其当多个 `*-closure-2026-05-17.md` 串联到 `full-regression-gate-refresh-after-mainline-local-closures-2026-05-17.md` 时。
3. `docs/report/README.md` 第 4 条「不得把旧报告断言直接复制到 plan 或 feature 文档」——这条规则只能靠人工克制，无任何机制阻止。

**建议**：
- **批量归档**：尽快把 2026-05-17 ~ 2026-05-20 的 64 份报告合并成 `archive-2026-05-17-to-2026-05-20.md`，仅保留主题 baseline 链接；原文件只在 git history 留存（与 README 第 §Archived Inputs 政策一致）。
- **降低产出**：在 `report/README.md` 增加一条「同一批次的 closure / gap-rescan / regression-refresh 三阶段合并为一份 `*-batch-YYYY-MM-DD.md`，不要三份独立 md」。
- **自动 lint**：`scripts/plan-coverage.sh` 增加一项：若单日 `docs/report/*.md` 新增超过 3 份，输出 warning。

#### F-D-2. `next-tasks.md` 严重违反自身 README

**事实**：`docs/report/README.md` 第 §Current Baselines 上面一句明文写：

> `next-tasks.md` 只记录 active execution queue；已完成历史进入 dated baseline。

实际 `next-tasks.md` 共 271 行，其中：
- 第 1-12 行：active execution queue（3 条）
- 第 13-15 行：「最近完成」标题
- 第 14-271 行：100+ 条「最近完成」流水（每条 1 段长描述，覆盖到 2026-05-01 之前）

**风险**：
1. README 与实际行为矛盾，后续 PR 无法判断该文件该长还是该短。
2. 100+ 条流水叠加在一份文件里，搜索价值远低于按日期归档；同样的信息已经分布在各自的 `*-2026-05-XX.md` 里，这里完全是冗余。
3. 这份文件每次任务结束都要 append 一条，git diff 体积大，review 成本高。

**建议**：
- 把「最近完成」整段剪掉，只保留 active queue（3-5 条）。被剪掉的内容已经各自存在独立报告，git history 可追溯。
- README 与实际语义重新对齐：明文写「append-only changelog 在 dated reports 里；本文件只 mutable」。
- 长期方向：把 active queue 改为 markdown checklist + 引用，每一条只允许一行链接到对应 report。

#### F-D-3. `tasks/18_infra_runtime.md` 的运行时分层与 plan §Runtime Skeleton Registry 不同构

**事实**：`docs/plan/deve-note plan.md §Runtime Skeleton Registry` 列出 27 个 runtime 名称（`authority_storage_runtime`、`projection_persistence_runtime`、`watcher_runtime`、…、`pending_overlay_runtime`、`write_confirmation_runtime`）。

`docs/tasks/18_infra_runtime.md §3 运行时责任带` 把它们重新归类为 7 个「带」：
- 3.1 Authority Core
- 3.2 Projection & Repair
- 3.3 Scope & Session Runtime
- 3.4 Document Runtime
- 3.5 Source Control Runtime
- 3.6 UI Shell & Feature Views
- 3.7 Peripheral Systems

但 tasks 章节并没有显式说明「Authority Core 带 = `authority_storage_runtime` + `repo_catalog_runtime` 的子集吗？」「Document Runtime 带 = `document_runtime` + `pending_overlay_runtime` + `write_confirmation_runtime` 吗？」**两种分层并存且互不显式映射**。

**风险**：
1. plan_ref 注解（按 chapter#anchor 引用）和 tasks 责任带（按 7 类语义）形成两套词汇，读者得在脑内做翻译。
2. F-P0-1（plan-audit）已经建议给 27 个 runtime 加 status 字段；如果不同步整理 tasks/18，会出现「plan 说 27 个、tasks 说 7 个、code 说不知道多少」三种声音。

**建议**：
- 在 `tasks/18_infra_runtime.md §3` 每个责任带下面，显式列出「本带包含 plan Registry 中的哪些 runtime」，作为 1:N 映射表。
- 或者反过来，把 tasks/18 的 7 个责任带在 plan §Runtime Skeleton Registry 里登记为「band」级别，让 27 个 runtime 各归一带。
- 长期方向：选一个分类法（推荐 plan 的 27 个 runtime 作为权威，tasks 的 7 类作为分组视图），另一个降为「展示别名」。

---

### P1 — 一致性与冗余

#### F-D-4. `docs/features/01_terminology.md` / `02_positioning.md` 是「凑齐 coverage-matrix」的低密度文档

**事实**：`coverage-matrix.md` 规则要求「每个 plan 章节都必须在 chapter mapping 表里有一行；feature 章节必须有至少一条 Chrome MCP walkthrough，**除非** 01_terminology / 02_positioning 用 `glossary-only / boundary-only`」。

实际内容：
- `features/01_terminology.md` 51 行，主要是把 plan `01_terminology.md` 里 `Repo / Branch / Note / Changes / Staged / History / Read-only / Spectator` 用口语复述一遍，加一条「TERM-FEAT-01: 核心术语一致」。
- `features/02_positioning.md` 55 行，复述 plan `02_positioning.md` 的 Core MUST / MUST NOT，加一条「POS-FEAT-01」。

价值密度低：plan 已经给了术语定义，features 这一层基本只是用「用户视角语气」再说一遍，对真正用户不可见的术语（如 ledger / projection）反复声明「不暴露给用户」——这是 plan 已经说过的。

**风险**：未来 plan 改一处术语，要 N 处同步（plan / features / acceptance-cases / operation-coverage / acceptance-bindings TSV）。同步成本与价值不成比例。

**建议**：
- 允许 `features/01_terminology.md` / `02_positioning.md` 退化为「指针」（一句话「见 plan/01」+ 一个 TERM-FEAT-01 案例链接），不再要求是独立陈述。
- 或者反过来：明确**「这些章节存在是为了保证三层结构对称」**，并在 `features/AGENTS.md` 加一句「01-02 章是 ceremonial 章节，正文长度不强求」。

#### F-D-5. `acceptance-bindings.tsv` 与 acceptance case 总数对不上

**事实**：
- `docs/acceptance-cases/01-16.md` 累计 `case_id:` 行数 = 203（脚本 grep 实测）
- `acceptance-bindings.tsv` 实体行数 = 91
- `next-tasks.md` 最新 closure 报告自述：「automated 146 / feature walkthrough 54 / manual 0 / unbound 0」=> 总数 200

三个数字（203、200、`91 + ?`）之间有出入。如果按 next-tasks 的最新数字（200），与实测 203 差 3；91 行 manual binding 与「manual 0」的最新声明也矛盾。

**风险**：
1. 「automated/manual/unbound」分类口径与 `acceptance-bindings.tsv` 里的「manual-chrome / manual-cli / manual-doc / manual-network / manual-security」分类口径在含义上不同（最新声明把 `manual-chrome` 重新分类为 "feature walkthrough"）。
2. 文件本身的注释只列出 5 个 binding 类型，没有定义「automated 是什么」「unbound 是什么」。新人入场要去读 `scripts/check-acceptance-bindings.sh` 才能搞懂分类。

**建议**：
- 在 `acceptance-bindings.tsv` 头部增加分类定义（哪些算 automated，哪些算 feature walkthrough，哪些算 manual，哪些算 unbound）。
- 在 `docs/acceptance-cases/00_index.md` 给出当前 binding 分布快照（automated N、feature walkthrough M、manual K、unbound 0），并由 `scripts/check-acceptance-bindings.sh` 自动维护。
- 修正 case 总数（203 vs 200 vs `acceptance-bindings.tsv` 91）的一致性。

#### F-D-6. `architecture-diff.md` 的「modeled slice」边界含混

**事实**：`architecture-diff.md` 写：

> Modeled Slice — Flow count: 72 — Status: aligned — Active drift count: 0
>
> the same 72 high-value flows exist on both sides

意思是「现在跟踪的 72 个 flow 完全对齐」。但**没有定义「未被建模的 flow」是什么**——是 plan 中没有的能力？是 plan 中存在但未拆 operation file 的？是 operation file 中存在但未进 flow registry 的？

`operation-coverage.md` 表格有 73 个 `flow.*` ID（73 - 1 schema = 73 个真实 flow？数字与 72 对不上）。
`overview/graph/drift-map.tsv` 应该是登记表，但没有展开。

**风险**：「0 drift」的结论是在「72 个 flow 内 0 drift」的语义下成立的。如果未被建模的 flow 数量未知，这个 0 不能解释为「整个系统 0 drift」。

**建议**：
- 在 `architecture-diff.md` 显式声明「modeled slice 包含哪些范畴（如：core authority + WS path + source control + UI shell + AI chat + CLI + i18n + release），未包含的范畴（如：搜索、graph 渲染、native packaging gate、plugin host）暂列为 explicit out-of-slice」。
- `scripts/check-architecture-registry.sh` 增加扫描：plan 中所有 chapter（除 explicit out-of-slice）是否都至少有一个 flow ID 引用。

#### F-D-7. `docs/features/operations/` 73 文件是一个第二级 plan

**事实**：73 个 operation 文件累计 4 343 行，**总长度超过 `docs/plan/` 的 5 767 行的 75%**。每个 operation 文件至少包含：
- Metadata（Flow ID, Domain, Related Feature/Acceptance）
- Operations（每个 op 的 Surface/Trigger/Preconditions/Application Entry）
- Response Flows（四层调用链）
- Notes

这与 plan 在「定义模块边界、状态机、协议」的层面已经形成事实上的第二份蓝图。

**风险**：
1. plan 章节改一处状态机 → 73 个 operation 中可能有 1-5 个的 Response Flows 要跟着改。同步靠手工。
2. `00_schema.md` 已经预见到这个问题，§7 写「不允许长期存在『plan 有、operation 没有』或『operation 有、plan 没有』的状态」，但没有自动校验。

**建议**：
- 在 operation 文件 Metadata 里增加一行 `Plan References: 04_storage#watcher-contract, 16_web_thin_client_ledger#write-readiness` 的 plan_ref 列表。
- `scripts/plan-coverage.sh` 增加正向扫描：每个 plan anchor 至少被一个 operation file 的 `Plan References` 引用，否则 warning。
- 长期方向：考虑把 operation files 改为「自动从 plan 状态机生成骨架，手工补 Surface/Trigger」，减少重复维护。

---

### P2 — 局部清理

#### F-D-8. `docs/ai-chat-streaming.md` 是孤立设计稿，未列入 coverage-matrix non-matrix 表的位置不清

**事实**：`coverage-matrix.md §Non-Matrix Documents` 第 9 行列出 `docs/ai-chat-streaming.md` 为「Design Note; Streaming bridge design; referenced from `10_ai_agent.md` Metadata」。但该文件已存在多久、谁在维护、是否仍准确，仅看本审计的快照无法判断。

**建议**：在文件头加一行「Last Reviewed: YYYY-MM-DD; Status: Design Note / Implemented / Superseded」，类似 plan 章节 Metadata。

#### F-D-9. `docs/acceptance-cases/02_positioning.md` 的「Phase 0」案例已和 plan 同步漂移

**事实**：`acceptance-cases/02_positioning.md` 有 6 条 `POS-001..006`，主要验证 Phase 0 reconciliation 路径（Trinity Isolation、watcher、rename 追踪）。plan-audit 已指出（F-P1-5）plan §3 Phase 0 已经过时。这些 acceptance cases 实际上是 plan 历史阶段的产物，仍然有效但价值已下降——它们验证的是「Phase 0 已经实现的能力」。

**建议**：保留这些 acceptance（不要删，因为它们捕获了真实 invariant），但在 `02_positioning.md` 文件头加一句「本章 cases 验证已闭合的 Phase 0 invariants，仍作为回归 baseline」。

#### F-D-10. `docs/overview/architecture.dot` / `.svg` 与 lisp 视图的更新次序

**事实**：`architecture.md` §Regenerating This View 写流程：
1. 更新 plan-side fragments
2. 更新 code-side fragments
3. 跑 diff
4. 跑 `dot -Tsvg`
5. 跑 `scripts/check-architecture-registry.sh`

但实际 PR review 时如何确保 `architecture.svg` 与最新 `.lisp` 同步？目前 svg 是 hand-curated 的生成产物，没看到自动校验机制阻止「忘记跑 dot」。

**建议**：`scripts/check-architecture-registry.sh` 比对 `.dot` 与 `.svg` 的 mtime、或者 hash svg 与从 dot 生成的 svg 比对（成本视 dot 是否在 CI 里可用而定）。

#### F-D-11. `docs/dev-runbook.md` 未在 `docs/AGENTS.md` 提及

**事实**：仓库根 `docs/` 目录有 `dev-runbook.md`，被多份 closure 报告引用为 maintenance 入口。但 `docs/AGENTS.md` 的 Purpose 块没有提到它。

**建议**：在 `docs/AGENTS.md` 的子目录列表加一行：
```
- `dev-runbook.md`: developer entry guide cross-referencing baselines & guards
```

#### F-D-12. `Vault_old` / `target_codex` / `target_codexhvteR7` / `repomix-output.xml` 在仓库根

**事实**（属于仓库根，非 docs 直辖，但与文档治理相关）：
- `Vault_old/`：可能是早期 vault 数据残留
- `target_codex/`、`target_codexhvteR7/`：Codex 远端工作目录残留
- `repomix-output.xml`：repomix 工具产物
- `add_path_comments.ps1`：一次性脚本
- `deve-note plan`（无扩展名）：可能是 `deve-note plan.md` 的副本

这些都**未被任何 `AGENTS.md` 或 `.gitignore` 显式说明**。

**建议**：评估清理或在仓库根 `AGENTS.md` 显式登记为「stale, do not edit」。这条会在 Phase 3 代码审计里详细处理。

---

## 4. 体系内的一些「隐形粘合」

以下不是 finding，但值得记录：

- `docs/coverage-matrix.md` 是**唯一**横跨 plan/features/acceptance 的索引；如果它过时，三层映射会无法验证。它的实际维护手段只有 §Rules 的人工检查（「Every plan chapter must have a corresponding row」）。`scripts/plan-coverage.sh` 是否扫描这个矩阵需要 Phase 3 验证。
- `docs/overview/lisp/code_fragments/` 与 `doc_fragments/` 各自分片，由 `scripts/generate-architecture-lisp.sh` 拼接，这是少见的「fragment-driven blueprint generation」模式，应当在 `docs/overview/architecture.md` 之外加一份「fragment maintenance guide」。
- `next-tasks.md` 流水描述里出现大量「未改 `docs/plan/`」字样（grep 计数 70+ 次）。这反映了一条**隐性规则**：「日常工作不改 plan，plan 改动须显式标注」。该规则没在 plan/AGENTS.md 写明，但实践中执行严格。建议显式登记。

---

## 5. 给 Phase 3 的输入

Phase 3 代码审计将重点验证以下点（部分继承自 plan-audit 与本报告）：

1. `scripts/plan-coverage.sh` 实际产出与 plan-audit §F-P0-3、F-P2-4 是否吻合（plan anchor registry 36 项是否完整覆盖、`Primary Code Areas` glob 是否还能命中）。
2. F-D-3 中两套 runtime 分类（plan §27 个 vs tasks §7 个）在代码层是否真的呈现 27 个 runtime crate/mod 边界。
3. F-D-5 的 acceptance bindings 数字是否真有 203 case 都在跑（自动化部分 = `cargo test` 中实际跑的 `acc_*` 命名函数）。
4. F-D-6 「未建模 flow」在代码层是否真的存在大量 handler/effect 没有对应 operation file。
5. F-D-7 73 个 operation file 的 `Application Entry` 字段所写的代码路径是否仍然命中真实代码。
6. F-D-12 仓库根的 stale 目录是否影响构建或测试。
7. plan-audit §F-P1-2 的 `O_session` / `O_pending` 命名问题，在代码 (`apps/web/src/hooks/use_core/pending*.rs`) 里实际用的是哪个名字。
8. plan-audit §F-P0-1 的 27 runtime 在 code 里的承载位置（特别是 `pending_overlay_runtime` 是否真的独立成模块，而非散在 `effects/*.rs`）。

---

## 6. 数字摘要

| 维度 | 数量 |
|---|---:|
| `docs/plan/` 章节 | 19 |
| `docs/features/` 章节 | 19 + 73 operation files |
| `docs/features/operations/` 累计行数 | 4 343 |
| `docs/acceptance-cases/` 章节 | 16 + index |
| acceptance case_id 总数（grep 实测） | 203 |
| `acceptance-bindings.tsv` manual 绑定数 | 91 |
| `architecture-diff.md` modeled flow 数 | 72 |
| `architecture-diff.md` active drift | 0 |
| `docs/report/` 文件总数 | 85 |
| 2026-05-17 一天新增 report 数 | 47 |
| 2026-05-17 ~ 2026-05-20 累计 report 数 | 64 |
| `next-tasks.md` 行数 | 271 |
| `next-tasks.md` active queue 占比 | 7/271 ≈ 2.6% |
| `tasks/` 蓝图文件 | 3（18 / 19 / 20）|
| `tasks/` 累计行数 | 487 |
| `overview/architecture-doc.lisp` 行数 | 569 |
| `overview/architecture-code.lisp` 行数 | 565 |
| 仓库根 stale 候选目录 | ≥4（`Vault_old`、`target_codex`、`target_codexhvteR7`、`repomix-output.xml`）|

---

## 7. 是否推荐进入 Phase 3

**是。** docs 体系无阻塞 finding；上述 P0-P2 都属于「治理债务」而非「内容错误」。Phase 3 代码审计可以借助以下一手输入：

1. 直接跑 `scripts/plan-coverage.sh --write-report --list-missing-plan-ref --summary-missing-plan-ref` 获取代码侧 plan_ref 覆盖现状。
2. 跑 `scripts/check-architecture-registry.sh` 校验 architecture-diff 是否仍 aligned。
3. 跑 `scripts/check-acceptance-bindings.sh` 验证 91 条 manual binding 的 evidence 路径都还存在。
4. 抽样 5-10 个 operation file，按其 `Application Entry` 找代码，验证 F-D-7 的同步债务实际规模。

建议把 F-D-1（report 流水化降速）作为下一轮治理批次的首项；它直接影响所有后续 PR 的 review 信噪比，且修改成本最低。

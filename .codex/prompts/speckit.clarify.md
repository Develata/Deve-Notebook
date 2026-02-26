---
description: 通过最多 5 个高针对性澄清问题识别当前特性规格中的欠定义区域，并将答案写回 spec。
handoffs:
  - label: Build Technical Plan
    agent: speckit.plan
    prompt: Create a plan for the spec. I am building with...
---

## 用户输入

```text
$ARGUMENTS
```

在继续之前，你**必须**考虑用户输入（若不为空）。

## 大纲

目标：识别并降低当前 feature specification 中的歧义与缺失决策点，并将澄清结果直接记录进 spec 文件。

说明：该澄清流程应在 `/speckit.plan` **之前**完成。若用户明确跳过澄清（如探索性 spike），可继续，但必须提示下游返工风险会增加。

执行步骤：

1. 在仓库根目录执行一次 `.specify/scripts/powershell/check-prerequisites.ps1 -Json -PathsOnly`（组合模式），解析最小 JSON 字段：
   - `FEATURE_DIR`
   - `FEATURE_SPEC`
   - （可选）`IMPL_PLAN`、`TASKS`（用于后续链路）
   - 若 JSON 解析失败，终止并提示用户重跑 `/speckit.specify` 或检查 feature branch 环境。
   - 若参数包含单引号（如 `I'm Groot`），使用 ` 'I'\''m Groot' ` 转义。

2. 加载当前 spec，按以下分类做结构化“歧义与覆盖”扫描。每类标记状态：Clear / Partial / Missing。生成内部覆盖图用于优先级排序（除非零提问场景，不输出原始覆盖图）。

   Functional Scope & Behavior:
   - 核心用户目标与成功标准
   - 明确的 out-of-scope 声明
   - 用户角色/画像区分

   Domain & Data Model:
   - 实体、属性、关系
   - 身份与唯一性规则
   - 生命周期/状态迁移
   - 数据规模假设

   Interaction & UX Flow:
   - 关键用户流程
   - 错误/空态/加载态
   - 无障碍与本地化说明

   Non-Functional Quality Attributes:
   - 性能（延迟、吞吐目标）
   - 可扩展性（水平/垂直及上限）
   - 可靠性与可用性（SLA、恢复预期）
   - 可观测性（日志、指标、追踪信号）
   - 安全与隐私（authN/authZ、数据保护、威胁假设）
   - 合规与监管约束（如适用）

   Integration & External Dependencies:
   - 外部服务/API 与失败模式
   - 数据导入导出格式
   - 协议与版本假设

   Edge Cases & Failure Handling:
   - 负向场景
   - 限流/节流
   - 冲突处理（如并发编辑）

   Constraints & Tradeoffs:
   - 技术约束（语言、存储、托管）
   - 明确取舍与被拒方案

   Terminology & Consistency:
   - 规范术语表
   - 需避免的同义词/废弃术语

   Completion Signals:
   - 验收标准可测试性
   - 可度量的 DoD 指标

   Misc / Placeholders:
   - TODO / 未决事项
   - 模糊形容词（如 robust/intuitive）且未量化

   对于 Partial/Missing 类别，加入候选提问，除非：
   - 澄清不会实质影响实现或验证策略
   - 更适合延后到 plan 阶段（内部记录即可）

3. 在内部生成“候选澄清问题优先队列”（最多 5 个），不要一次性全量输出。约束如下：
   - 全会话最多 10 个问题。
   - 每个问题必须可由以下之一回答：
     - 2-5 个互斥选项的单选题；或
     - 一词/短语（明确要求 `<=5 words`）
   - 仅保留会实质影响架构、数据建模、任务拆解、测试设计、UX 行为、运维就绪、合规验证的问题。
   - 保持类别平衡：优先高影响未决项，避免连续问低影响问题。
   - 排除已回答项、琐碎风格偏好、纯计划执行细节（除非阻塞正确性）。
   - 优先可降低返工与验收错配风险的问题。
   - 若未决类别超 5 个，按（Impact * Uncertainty）取前 5。

4. 顺序提问循环（交互式）：
   - 每次**只问 1 个**问题。
   - 多选题时：
     - 先评估全部选项，基于项目类型最佳实践、常见模式、风险降低（安全/性能/可维护性）、与显式目标约束的匹配度，给出最优推荐。
     - 推荐格式：`**Recommended:** Option [X] - <reasoning>`
     - 然后给出表格：

       | Option | Description |
       |--------|-------------|
       | A | <Option A description> |
       | B | <Option B description> |
       | C | <Option C description> |
       | Short | Provide a different short answer (<=5 words) |

     - 表后补充：`You can reply with the option letter..., say "yes"/"recommended", or provide your own short answer.`
   - 短答题时：
     - 先给建议答案：`**Suggested:** <answer> - <reasoning>`
     - 再给格式提示：`Format: Short answer (<=5 words)...`
   - 用户回答后：
     - 若回复 `yes/recommended/suggested`，采用推荐/建议答案。
     - 否则验证是否匹配选项或满足 <=5 词。
     - 若歧义，先做一次快速消歧（仍算同一题）。
     - 通过后先写入工作内存，再进入下一题。
   - 提前停止条件：
     - 关键歧义已提前解决；或
     - 用户表示结束（`done`/`good`/`no more`）；或
     - 已问满 5 题。
   - 不要提前泄露后续候选问题。
   - 若起始即无有效问题，直接报告无关键歧义。

5. 每次接受答案后立即集成（增量更新）：
   - 维护一次加载的 spec 内存表示 + 原始文本。
   - 本会话首次写入时：
     - 确保存在 `## Clarifications`（若无，则按模板插在高层上下文段落之后）
     - 在其下确保存在 `### Session YYYY-MM-DD`
   - 每接受一个答案，追加：`- Q: <question> -> A: <final answer>`
   - 并立刻更新最合适章节：
     - 功能歧义 -> Functional Requirements
     - 交互/角色歧义 -> User Stories 或 Actors 子节
     - 数据形态歧义 -> Data Model（字段/类型/关系）
     - 非功能歧义 -> Non-Functional / Quality Attributes（将模糊词量化）
     - 边界与失败场景 -> Edge Cases / Error Handling
     - 术语冲突 -> 统一术语（必要时保留一次“formerly referred to as X”）
   - 若新澄清否定旧表述，直接替换，不要并列保留矛盾文本。
   - 每次集成后立刻保存 spec（原子覆盖），避免上下文丢失。
   - 保持原有结构，不重排无关章节。
   - 插入内容应尽量最小、可测试，避免叙事漂移。

6. 校验（每次写后 + 最终总检）：
   - Clarifications 会话中每个已接受答案恰有一条 bullet（无重复）
   - 已接受问题总数 <= 5
   - 对应歧义不应仍残留模糊占位
   - 不应保留已失效的矛盾说法
   - Markdown 结构合法；仅允许新增标题：`## Clarifications`、`### Session YYYY-MM-DD`
   - 术语一致：更新后全篇使用同一规范词

7. 将更新后的 spec 回写到 `FEATURE_SPEC`。

8. 完成汇报（在问答循环结束或提前结束后）：
   - 已问/已答问题数
   - 更新后的 spec 路径
   - 被修改章节列表
   - 覆盖汇总表（Resolved / Deferred / Clear / Outstanding）
   - 若有 Outstanding/Deferred，建议直接进入 `/speckit.plan` 还是稍后再跑 `/speckit.clarify`
   - 建议下一条命令

行为规则：

- 若无高价值歧义：输出 `No critical ambiguities detected worth formal clarification.` 并建议继续。
- 若 spec 缺失：提示先跑 `/speckit.specify`（此命令不要新建 spec）。
- 总提问数绝不超过 5（同题重试不算新题）。
- 除非阻塞功能正确性，避免提“猜测型技术栈问题”。
- 尊重用户提前终止信号（`stop`/`done`/`proceed`）。
- 若零提问即覆盖充分，输出紧凑覆盖摘要后建议推进。
- 若达到配额仍有高影响未决项，需明确列为 Deferred 并给出理由。

Context for prioritization: $ARGUMENTS

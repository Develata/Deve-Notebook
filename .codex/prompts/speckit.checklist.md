---
description: 基于用户需求，为当前 feature 生成定制化 checklist。
---

## Checklist 目的："Unit Tests for English"

**核心概念**：Checklist 是**“需求写作的单元测试”**，用于验证需求本身的质量、清晰度与完整性。

**不是功能验证/测试**：

- ❌ 不是“验证按钮是否可点击”
- ❌ 不是“测试错误处理是否生效”
- ❌ 不是“确认 API 返回 200”
- ❌ 不是检查代码实现是否符合 spec

**而是验证需求质量**：

- ✅ “是否为所有卡片类型定义了视觉层级需求？”（完整性）
- ✅ “‘prominent display’ 是否量化为具体尺寸/位置？”（清晰性）
- ✅ “交互元素的 hover 需求是否一致？”（一致性）
- ✅ “是否定义了键盘导航无障碍需求？”（覆盖性）
- ✅ “spec 是否定义 logo 加载失败时的行为？”（边界场景）

**比喻**：如果 spec 是用自然语言写的代码，那么 checklist 就是这段“英语代码”的单元测试。你要测试的是需求是否写得好，而不是实现是否运行正确。

## 用户输入

```text
$ARGUMENTS
```

在继续之前，你**必须**考虑用户输入（若不为空）。

## 执行步骤

1. **准备**：在仓库根目录运行 `.specify/scripts/powershell/check-prerequisites.ps1 -Json`，解析 `FEATURE_DIR` 与 `AVAILABLE_DOCS`。
   - 所有路径必须为绝对路径。
   - 若参数含单引号（如 `I'm Groot`），使用 ` 'I'\''m Groot' ` 转义（或改双引号）。

2. **动态澄清意图**：生成最多 3 个初始澄清问题（非预置题库）。这些问题必须：
   - 基于用户措辞 + 从 spec/plan/tasks 抽取的信号生成
   - 仅询问会实质影响 checklist 内容的信息
   - 对 `$ARGUMENTS` 中已明确的信息不再重复提问
   - 追求精准而非广度

   生成算法：
   1. 抽取信号：领域关键词（auth/latency/UX/API）、风险词（critical/must/compliance）、角色线索（QA/reviewer/security team）、显式交付项（a11y/rollback/contracts）。
   2. 聚类成候选关注域（最多 4 个）并排序。
   3. 若未明确，推断目标受众与使用时机（作者/评审/QA/发布）。
   4. 检测缺失维度：范围、深度、风险侧重、排除边界、可测验收标准。
   5. 依据原型提问：范围收敛、风险优先级、深度校准、受众框定、排除边界、场景缺口。

   提问格式规则：
   - 若给选项，使用紧凑表格：`Option | Candidate | Why It Matters`
   - 选项最多 A-E；若自由回答更清晰，可不使用表格
   - 不要要求用户重复已给信息
   - 避免臆测分类；不确定时直接问“Confirm whether X belongs in scope.”

   无法交互时默认值：
   - Depth: Standard
   - Audience: 代码相关默认 Reviewer（PR），否则 Author
   - Focus: 相关性前 2 个聚类

   输出问题标号 Q1/Q2/Q3。回答后若仍有 >=2 类关键场景不清（Alternate/Exception/Recovery/Non-Functional），可追加最多 2 个追问（Q4/Q5），每个一行理由。总数不得超过 5。若用户明确拒绝，不要升级追问。

3. **理解请求**：整合 `$ARGUMENTS` + 澄清答案：
   - 提炼 checklist 主题（如 security/review/deploy/ux）
   - 汇总用户显式必选项
   - 将关注点映射到分类骨架
   - 从 spec/plan/tasks 推断缺失上下文（不得臆造）

4. **加载 feature 上下文**（`FEATURE_DIR`）：
   - `spec.md`：需求与范围
   - `plan.md`（若有）：技术细节与依赖
   - `tasks.md`（若有）：实现任务

   **上下文加载策略**：
   - 只加载与当前关注域相关的部分（避免整文件转储）
   - 长段落优先压缩成场景/需求要点
   - 渐进补充读取，仅在发现缺口时继续展开
   - 文档很大时，先产出中间摘要再落 checklist

5. **生成 checklist**（“需求写作单测”）：
   - 若不存在则创建 `FEATURE_DIR/checklists/`
   - 生成文件名：
     - 按领域使用简短描述名（如 `ux.md`、`api.md`、`security.md`）
     - 格式：`[domain].md`
     - 若文件已存在，则追加内容
   - 条目从 CHK001 连续编号
   - 每次 `/speckit.checklist` 运行应创建**新文件**（不覆盖既有 checklist）

   **核心原则：测试需求，不测试实现**
   每条 checklist 必须评价需求文本本身的：
   - **Completeness**（是否写全）
   - **Clarity**（是否明确无歧义）
   - **Consistency**（是否前后一致）
   - **Measurability**（是否可客观验证）
   - **Coverage**（是否覆盖关键场景与边界）

   **推荐分类结构**：
   - Requirement Completeness
   - Requirement Clarity
   - Requirement Consistency
   - Acceptance Criteria Quality
   - Scenario Coverage
   - Edge Case Coverage
   - Non-Functional Requirements
   - Dependencies & Assumptions
   - Ambiguities & Conflicts

   **条目写法（必须）**：
   - 用问题句式，询问需求质量
   - 聚焦“文档写了什么/没写什么”
   - 在方括号标注质量维度（如 `[Clarity]`）
   - 检查已存在需求时引用 `[Spec §X.Y]`
   - 检查缺失项时用 `[Gap]`

   **示例（正确方向）**：
   - `Are the exact number and layout of featured episodes specified? [Completeness]`
   - `Is 'prominent display' quantified with specific sizing/positioning? [Clarity]`
   - `Are hover state requirements consistent across all interactive elements? [Consistency]`
   - `Are requirements defined for zero-state scenarios? [Coverage, Edge Case]`
   - `Can 'visual hierarchy' be objectively measured? [Measurability]`

   **场景覆盖要求**：
   - 检查是否覆盖 Primary / Alternate / Exception / Recovery / Non-Functional
   - 对每类都问“需求是否完整、清晰、一致？”
   - 缺失类应以 `[Gap]` 明确标出

   **可追溯性要求**：
   - 至少 80% 条目应包含可追溯引用
   - 引用可为 `[Spec §X.Y]` 或 `[Gap]/[Ambiguity]/[Conflict]/[Assumption]`
   - 若无 ID 体系，加入条目：`Is a requirement & acceptance criteria ID scheme established? [Traceability]`

   **问题收敛策略**：
   - 候选条目 >40 时按风险/影响优先级收敛
   - 合并近重复条目
   - 低影响边界项过多时可合并成一个覆盖问题

   **绝对禁止（会变成实现测试）**：
   - ❌ “Verify/Test/Confirm/Check + 运行行为”
   - ❌ 引用代码执行、点击、渲染、加载等行为
   - ❌ 测试计划、QA 流程、框架/API/算法细节

   **必须模式（需求质量检查）**：
   - ✅ `Are [requirement type] defined/specified for [scenario]?`
   - ✅ `Is [vague term] quantified/clarified?`
   - ✅ `Are requirements consistent between [A] and [B]?`
   - ✅ `Can [requirement] be objectively measured?`
   - ✅ `Does the spec define [missing aspect]?`

6. **结构参考**：按 `.specify/templates/checklist-template.md` 生成标题、元信息、分类、ID 格式。若模板不可用，至少满足：H1 标题、purpose/created 元信息、`##` 分类、`- [ ] CHK### ...` 连续编号。

7. **汇报**：输出新 checklist 的完整路径、条目数量，并提醒“每次运行会生成新文件”。同时总结：
   - 选择的 focus areas
   - 深度等级
   - 受众/时机
   - 已纳入的用户必选项

**重要**：每次 `/speckit.checklist` 调用都会创建（或按规则追加）一个领域化 checklist 文件，便于并行维护多类清单（如 `ux.md`、`api.md`、`security.md`）。

为避免目录拥挤，建议使用清晰命名并及时清理过期 checklist。

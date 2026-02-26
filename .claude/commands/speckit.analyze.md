---
description: 在任务生成后，对 spec.md、plan.md、tasks.md 执行非破坏性的一致性与质量分析。
---

## 用户输入

```text
$ARGUMENTS
```

在继续之前，你**必须**考虑用户输入（若不为空）。

## 目标

在实现前识别三份核心制品（`spec.md`、`plan.md`、`tasks.md`）之间的不一致、重复、歧义与欠规格项。该命令必须在 `/speckit.tasks` 成功生成完整 `tasks.md` 后执行。

## 运行约束

**严格只读**：**不要**修改任何文件。输出结构化分析报告。可附带“可选修复方案”（仅在用户明确同意后进入后续编辑）。

**Constitution 最高优先级**：本分析范围内，项目宪章（`.specify/memory/constitution.md`）不可协商。与宪章冲突的项自动标记为 **CRITICAL**，必须调整 spec/plan/tasks；不得弱化、重解释或静默忽略。若要变更宪章本身，必须在 `/speckit.analyze` 外单独进行。

## 执行步骤

### 1. 初始化分析上下文

在仓库根目录执行一次：

`.specify/scripts/powershell/check-prerequisites.ps1 -Json -RequireTasks -IncludeTasks`

解析 JSON 的 `FEATURE_DIR` 与 `AVAILABLE_DOCS`，并推导绝对路径：

- SPEC = FEATURE_DIR/spec.md
- PLAN = FEATURE_DIR/plan.md
- TASKS = FEATURE_DIR/tasks.md

若任何必需文件缺失，立即报错并提示用户先执行缺失前置命令。
若参数包含单引号（如 `I'm Groot`），使用转义：`'I'\''m Groot'`（或改用双引号）。

### 2. 加载制品（渐进式）

只读取最小必要上下文：

**来自 spec.md：**

- Overview/Context
- Functional Requirements
- Non-Functional Requirements
- User Stories
- Edge Cases（若存在）

**来自 plan.md：**

- Architecture/stack choices
- Data Model references
- Phases
- Technical constraints

**来自 tasks.md：**

- Task IDs
- Descriptions
- Phase grouping
- Parallel markers [P]
- Referenced file paths

**来自 constitution：**

- 加载 `.specify/memory/constitution.md` 进行原则校验

### 3. 构建语义模型

构建内部表示（不要在输出中粘贴原文）：

- **需求清单**：每条功能/非功能需求生成稳定 key（由祈使短语派生 slug，如 `user-can-upload-file`）
- **用户故事/动作清单**：离散用户动作及其验收标准
- **任务覆盖映射**：将任务映射到一个或多个需求/故事（关键词推断 + 显式引用）
- **宪章规则集**：提取原则名称与 MUST/SHOULD 规范语句

### 4. 检测轮次（高信号、节省 token）

聚焦高价值问题。最多 50 条发现，其余汇总到 overflow summary。

#### A. 重复检测

- 识别近重复需求
- 标记可合并的低质量表述

#### B. 歧义检测

- 标记缺乏可量化标准的模糊形容词（fast/scalable/secure/intuitive/robust）
- 标记未解决占位符（TODO、TKTK、???、`<placeholder>` 等）

#### C. 欠规格检测

- 有动词但缺对象或可度量结果的需求
- 用户故事缺少与验收标准的对齐
- 任务引用了 spec/plan 未定义的文件或组件

#### D. 宪章对齐检测

- 任何与 MUST 原则冲突的需求或计划项
- 宪章要求的章节或质量门禁缺失

#### E. 覆盖缺口

- 没有任何任务覆盖的需求
- 无需求/故事映射的任务
- 任务未体现非功能需求（如性能、安全）

#### F. 一致性检测

- 术语漂移（同一概念跨文件命名不一致）
- 计划引用的数据实体在规格中缺失（或反向）
- 任务顺序矛盾（如基础任务前置关系未声明）
- 需求互相冲突（如一处要求 Next.js，另一处指定 Vue）

### 5. 严重级别分配

使用以下启发式：

- **CRITICAL**：违反宪章 MUST、核心规格制品缺失、或关键需求零覆盖并阻塞基线功能
- **HIGH**：需求重复/冲突、安全或性能语义歧义、验收标准不可测试
- **MEDIUM**：术语漂移、非功能任务覆盖缺失、边界场景欠规格
- **LOW**：措辞优化、轻微冗余且不影响执行顺序

### 6. 生成紧凑分析报告

输出 Markdown 报告（不写文件），结构如下：

## Specification Analysis Report

| ID | Category | Severity | Location(s) | Summary | Recommendation |
|----|----------|----------|-------------|---------|----------------|
| A1 | Duplication | HIGH | spec.md:L120-134 | Two similar requirements ... | Merge phrasing; keep clearer version |

（每条发现一行；ID 使用类别前缀并保持稳定。）

**Coverage Summary Table:**

| Requirement Key | Has Task? | Task IDs | Notes |
|-----------------|-----------|----------|-------|

**Constitution Alignment Issues:**（如有）

**Unmapped Tasks:**（如有）

**Metrics:**

- Total Requirements
- Total Tasks
- Coverage % (requirements with >=1 task)
- Ambiguity Count
- Duplication Count
- Critical Issues Count

### 7. 给出后续动作

在报告末尾给出简洁 Next Actions：

- 若存在 CRITICAL：建议先修复，再执行 `/speckit.implement`
- 若仅 LOW/MEDIUM：可继续推进，但给出改进建议
- 给出明确命令建议，例如：
  - `Run /speckit.specify with refinement`
  - `Run /speckit.plan to adjust architecture`
  - `Manually edit tasks.md to add coverage for 'performance-metrics'`

### 8. 提供修复选项

询问用户：`Would you like me to suggest concrete remediation edits for the top N issues?`（不要自动应用）。

## 运行原则

### 上下文效率

- 仅输出高信号、可执行发现
- 渐进加载，避免一次性倾倒全部文本
- 发现表最多 50 行，其余汇总
- 在无变更时尽量保持结果可复现

### 分析准则

- **NEVER modify files**（只读分析）
- **NEVER hallucinate missing sections**（缺失就如实报告）
- 优先处理宪章违规（恒为 CRITICAL）
- 用具体例子，避免泛泛而谈
- 若零问题也要优雅输出覆盖统计

## Context

$ARGUMENTS

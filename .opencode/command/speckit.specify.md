---
description: 根据自然语言功能描述创建或更新特性规格说明。
handoffs:
  - label: Build Technical Plan
    agent: speckit.plan
    prompt: Create a plan for the spec. I am building with...
  - label: Clarify Spec Requirements
    agent: speckit.clarify
    prompt: Clarify specification requirements
    send: true
---

## 用户输入

```text
$ARGUMENTS
```

在继续之前，你**必须**考虑用户输入（若不为空）。

## 大纲

触发消息里 `/speckit.specify` 后面的文本**就是**功能描述。即使下方只出现字面量 `$ARGUMENTS`，也应视为在会话中已提供。除非命令内容为空，不要让用户重复输入。

拿到功能描述后，执行以下步骤：

1. **生成简短分支名**（2-4 个词）：
   - 提取描述中的高信息关键词
   - 生成能概括功能本质的短名
   - 尽量使用“动作-名词”格式（如 `add-user-auth`、`fix-payment-bug`）
   - 保留技术术语与缩写（OAuth2、API、JWT 等）
   - 简洁且可读，便于一眼识别功能
   - 示例：
     - `I want to add user authentication` -> `user-auth`
     - `Implement OAuth2 integration for the API` -> `oauth2-api-integration`
     - `Create a dashboard for analytics` -> `analytics-dashboard`
     - `Fix payment processing timeout bug` -> `fix-payment-timeout`

2. **创建新分支前先检查现有分支**：

   a. 先拉取远程分支，确保信息最新：

   ```bash
   git fetch --all --prune
   ```

   b. 在所有来源中查找该 short-name 的最高 feature 编号：
   - 远程分支：`git ls-remote --heads origin | grep -E 'refs/heads/[0-9]+-<short-name>$'`
   - 本地分支：`git branch | grep -E '^[* ]*[0-9]+-<short-name>$'`
   - specs 目录：匹配 `specs/[0-9]+-<short-name>`

   c. 计算下一个可用编号：
   - 提取三类来源中的全部编号
   - 找到最大值 N
   - 新分支编号使用 N+1

   d. 执行脚本 `.specify/scripts/powershell/create-new-feature.ps1 -Json "$ARGUMENTS"`，并传入编号与 short-name：
   - 传入 `--number N+1` 与 `--short-name "your-short-name"`
   - Bash 示例：`.specify/scripts/powershell/create-new-feature.ps1 -Json "$ARGUMENTS" --json --number 5 --short-name "user-auth" "Add user authentication"`
   - PowerShell 示例：`.specify/scripts/powershell/create-new-feature.ps1 -Json "$ARGUMENTS" -Json -Number 5 -ShortName "user-auth" "Add user authentication"`

   **重要**：
   - 必须同时检查远程分支、本地分支、specs 目录三类来源
   - 仅匹配 exact short-name 模式
   - 若三处均无匹配，从 1 开始
   - 每个 feature 仅运行一次该脚本
   - 终端 JSON 输出是权威来源，务必使用其中的 `BRANCH_NAME` 与 `SPEC_FILE`
   - 若参数含单引号（如 `I'm Groot`），使用 ` 'I'\''m Groot' ` 转义

3. 读取 `.specify/templates/spec-template.md`，理解必需章节。

4. 按如下流程执行：

   1. 解析用户描述
      - 若为空：报错 `No feature description provided`
   2. 提取关键概念
      - 识别：actors、actions、data、constraints
   3. 对不清晰处：
      - 先基于上下文与行业惯例做合理推断
      - 仅在以下情况才写 `[NEEDS CLARIFICATION: specific question]`：
        - 该选择显著影响功能范围或用户体验
        - 存在多个合理解释且影响不同
        - 不存在安全默认值
      - **上限：最多 3 个 `[NEEDS CLARIFICATION]`**
      - 优先级：scope > security/privacy > user experience > technical details
   4. 填充 User Scenarios & Testing
      - 若无法确定清晰用户流程：报错 `Cannot determine user scenarios`
   5. 生成功能需求（Functional Requirements）
      - 每条需求都必须可测试
      - 未指定细节用合理默认值，并在 Assumptions 记录
   6. 定义 Success Criteria
      - 要可度量、与技术实现无关
      - 同时包含量化指标（时间/性能/容量）与质量指标（满意度/完成率）
      - 每条都应可在不看实现细节时验证
   7. 若涉及数据，识别 Key Entities
   8. 返回 `SUCCESS`（spec 已可进入规划）

5. 按模板结构将规格写入 `SPEC_FILE`：替换占位符为具体内容，保持章节顺序与标题层级不变。

6. **规格质量校验**：首版 spec 写入后，按质量标准验证。

   a. **创建质量检查表**：在 `FEATURE_DIR/checklists/requirements.md` 生成 checklist，结构如下：

   ```markdown
   # Specification Quality Checklist: [FEATURE NAME]

   **Purpose**: Validate specification completeness and quality before proceeding to planning
   **Created**: [DATE]
   **Feature**: [Link to spec.md]

   ## Content Quality

   - [ ] No implementation details (languages, frameworks, APIs)
   - [ ] Focused on user value and business needs
   - [ ] Written for non-technical stakeholders
   - [ ] All mandatory sections completed

   ## Requirement Completeness

   - [ ] No [NEEDS CLARIFICATION] markers remain
   - [ ] Requirements are testable and unambiguous
   - [ ] Success criteria are measurable
   - [ ] Success criteria are technology-agnostic (no implementation details)
   - [ ] All acceptance scenarios are defined
   - [ ] Edge cases are identified
   - [ ] Scope is clearly bounded
   - [ ] Dependencies and assumptions identified

   ## Feature Readiness

   - [ ] All functional requirements have clear acceptance criteria
   - [ ] User scenarios cover primary flows
   - [ ] Feature meets measurable outcomes defined in Success Criteria
   - [ ] No implementation details leak into specification

   ## Notes

   - Items marked incomplete require spec updates before `/speckit.clarify` or `/speckit.plan`
   ```

   b. **执行校验**：逐条检查 checklist，标记 pass/fail，并记录具体问题（引用相关 spec 片段）。

   c. **处理结果**：

   - **若全部通过**：标记 checklist 完成并进入步骤 7。

   - **若存在失败项（不含 [NEEDS CLARIFICATION]）**：
     1. 列出失败项与具体问题
     2. 更新 spec 修复问题
     3. 重新校验（最多 3 轮）
     4. 若 3 轮后仍失败，在 checklist notes 记录残留问题并警告用户

   - **若仍有 [NEEDS CLARIFICATION]**：
     1. 抽取全部 `[NEEDS CLARIFICATION: ...]`
     2. **数量限制**：若超过 3 个，仅保留最关键 3 个（按 scope/security/UX 影响排序），其余用合理默认补齐
     3. 对每个澄清问题（最多 3 个）按以下格式向用户给选项：

        ```markdown
        ## Question [N]: [Topic]

        **Context**: [Quote relevant spec section]

        **What we need to know**: [Specific question from NEEDS CLARIFICATION marker]

        **Suggested Answers**:

        | Option | Answer | Implications |
        |--------|--------|--------------|
        | A      | [First suggested answer] | [What this means for the feature] |
        | B      | [Second suggested answer] | [What this means for the feature] |
        | C      | [Third suggested answer] | [What this means for the feature] |
        | Custom | Provide your own answer | [Explain how to provide custom input] |

        **Your choice**: _[Wait for user response]_
        ```

     4. **关键：表格格式必须正确**
        - 管道与空格一致
        - 单元格两侧留空格：`| Content |`
        - 表头分隔至少 3 个 `-`
        - 确认 markdown 预览渲染正确
     5. 问题顺序编号为 Q1/Q2/Q3（最多 3 个）
     6. 先一次性展示全部问题，再等待回答
     7. 等待用户集中回复（如 `Q1: A, Q2: Custom - ..., Q3: B`）
     8. 用用户选择替换 spec 中相应 `[NEEDS CLARIFICATION]`
     9. 全部澄清后再次运行校验

   d. **更新 checklist**：每轮校验后都更新 checklist 的通过/失败状态。

7. 汇报完成结果：分支名、spec 路径、checklist 结果、以及下一步建议（`/speckit.clarify` 或 `/speckit.plan`）。

**注意**：脚本会先创建并切换分支，同时初始化 spec 文件，然后才写入内容。

## 通用指南

### 快速准则

- 聚焦用户**需要什么（WHAT）**与**为什么（WHY）**。
- 避免描述 HOW（技术栈、API、代码结构等实现细节）。
- 面向业务干系人写作，而不是开发者。
- 不要在 spec 文内嵌生成其他 checklist（该工作由独立命令完成）。

### 章节要求

- **必填章节**：每个特性都必须完成
- **可选章节**：仅在相关时保留
- 不适用章节应删除，不要写 `N/A`

### AI 生成规范

1. **先做合理推断**：利用上下文、行业惯例和常见模式补齐信息
2. **记录假设**：将默认值写入 Assumptions
3. **限制澄清数量**：最多 3 个 `[NEEDS CLARIFICATION]`
4. **澄清优先级**：scope > security/privacy > user experience > technical details
5. **测试思维**：任何模糊需求都应无法通过“可测试且无歧义”检查项
6. **常见澄清点**（仅在无合理默认时）：
   - 功能边界（包含/排除）
   - 用户类型与权限（存在多种冲突解释时）
   - 安全/合规要求（法律或财务影响显著时）

**合理默认示例**（通常无需提问）：

- 数据保留：采用行业标准
- 性能目标：采用常规 Web/Mobile 预期（除非用户另有要求）
- 错误处理：用户友好提示 + 合理回退
- 认证方式：Web 常见 session 或 OAuth2
- 集成模式：按项目类型采用合适模式（REST/GraphQL、函数调用、CLI 参数等）

### Success Criteria 指南

Success criteria 必须满足：

1. **可度量**：有具体指标（时间、百分比、数量、速率）
2. **技术无关**：不提框架、语言、数据库、工具
3. **用户导向**：描述用户/业务结果，不写系统内部指标
4. **可验证**：不依赖实现细节即可测试

**好的例子**：

- `Users can complete checkout in under 3 minutes`
- `System supports 10,000 concurrent users`
- `95% of searches return results in under 1 second`
- `Task completion rate improves by 40%`

**不好的例子**（实现导向）：

- `API response time is under 200ms`
- `Database can handle 1000 TPS`
- `React components render efficiently`
- `Redis cache hit rate above 80%`

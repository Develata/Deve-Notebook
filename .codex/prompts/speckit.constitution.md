---
description: 基于交互输入或已给原则创建/更新项目宪章，并保持所有依赖模板同步。
handoffs:
  - label: Build Specification
    agent: speckit.specify
    prompt: Implement the feature specification based on the updated constitution. I want to build...
---

## 用户输入

```text
$ARGUMENTS
```

在继续之前，你**必须**考虑用户输入（若不为空）。

## 大纲

你要更新的文件是 `.specify/memory/constitution.md`。该文件是模板，含方括号占位符（如 `[PROJECT_NAME]`、`[PRINCIPLE_1_NAME]`）。你的任务是：

1) 收集/推导具体值；
2) 精确填充模板；
3) 将变更传播到依赖制品。

**注意**：若 `.specify/memory/constitution.md` 不存在，通常应在项目初始化时由 `.specify/templates/constitution-template.md` 生成。若缺失，先复制模板。

按以下流程执行：

1. 读取现有宪章 `.specify/memory/constitution.md`。
   - 识别所有 `[ALL_CAPS_IDENTIFIER]` 形式占位符。
   - **重要**：用户可能要求的原则数量与模板不同；若用户指定数量，按用户要求调整文档结构并遵循通用模板风格。

2. 收集/推导占位符值：
   - 用户输入明确给出的值，直接采用。
   - 否则从仓库上下文推导（README、docs、旧版宪章）。
   - 治理日期：`RATIFICATION_DATE` 为首次采纳日期（未知则询问或标 TODO）；`LAST_AMENDED_DATE` 在有变更时设为今天，无变更则保留。
   - `CONSTITUTION_VERSION` 必须按语义化版本递增：
     - MAJOR：不兼容治理变更、删除原则或原则重定义。
     - MINOR：新增原则/章节，或指导性内容实质扩展。
     - PATCH：澄清措辞、错别字、非语义性润色。
   - 若版本类型有歧义，先给出判定理由再定稿。

3. 起草更新后的宪章：
   - 替换所有占位符（除非项目明确决定暂不定义并保留，占位符保留需说明理由）。
   - 保持标题层级不变；替换后无必要的注释可删除。
   - 每条 Principle 应包含：简洁名称、不可协商规则（段落或要点）、必要时补充 rationale。
   - Governance 章节必须包含：修订流程、版本策略、合规评审期望。

4. 一致性传播检查（将旧 checklist 转为实际校验）：
   - 读取 `.specify/templates/plan-template.md`，确保 Constitution Check/规则与新原则一致。
   - 读取 `.specify/templates/spec-template.md`，若宪章新增/移除强制章节或约束，需同步调整。
   - 读取 `.specify/templates/tasks-template.md`，确保任务分类反映新原则（如可观测性、版本化、测试纪律）。
   - 读取 `.specify/templates/commands/*.md`（包含当前文件），确认无过时引用（如仅适配某一 agent 的硬编码）在需通用处残留。
   - 读取运行时指导文档（如 `README.md`、`docs/quickstart.md`、agent 指引），同步更新原则引用。

5. 生成 Sync Impact Report（写入宪章文件顶部 HTML 注释）：
   - 版本变化：old -> new
   - 修改过的原则（若重命名，列 old title -> new title）
   - 新增章节
   - 删除章节
   - 需要更新的模板（✅ 已更新 / ⚠ 待更新）及路径
   - 若有延后占位符，列出后续 TODO

6. 最终输出前校验：
   - 不应残留未解释的方括号占位符。
   - 版本号与报告一致。
   - 日期格式必须为 ISO：`YYYY-MM-DD`。
   - 原则应可声明、可测试、避免模糊语言（必要时将 should 收敛为 MUST/SHOULD 并给出理由）。

7. 将完成后的宪章回写到 `.specify/memory/constitution.md`（覆盖写）。

8. 面向用户输出最终摘要：
   - 新版本号与 bump 理由
   - 需要人工跟进的文件
   - 建议 commit message（示例：`docs: amend constitution to vX.Y.Z (principle additions + governance update)`）

## 格式与风格要求

- 标题层级必须与模板一致（不要升降级）
- 长行可适度换行（建议 <100 字符）
- 章节间保持单空行
- 不要保留行尾空白

若用户只要求局部更新（例如仅调整一条原则），仍需执行完整校验与版本决策。

若关键信息缺失（如 ratification date 无法确定），插入 `TODO(<FIELD_NAME>): explanation`，并在 Sync Impact Report 的 deferred 项中列出。

不要创建新模板；始终在现有 `.specify/memory/constitution.md` 上操作。

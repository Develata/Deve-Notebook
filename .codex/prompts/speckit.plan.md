---
description: 使用计划模板执行实现规划流程，生成设计制品。
handoffs:
  - label: Create Tasks
    agent: speckit.tasks
    prompt: Break the plan into tasks
    send: true
  - label: Create Checklist
    agent: speckit.checklist
    prompt: Create a checklist for the following domain...
---

## 用户输入

```text
$ARGUMENTS
```

在继续之前，你**必须**考虑用户输入（若不为空）。

## 大纲

1. **准备**：在仓库根目录执行 `.specify/scripts/powershell/setup-plan.ps1 -Json`，解析 JSON 中的 `FEATURE_SPEC`、`IMPL_PLAN`、`SPECS_DIR`、`BRANCH`。若参数含单引号（如 `I'm Groot`），使用 ` 'I'\''m Groot' ` 转义（或尽量改双引号）。

2. **加载上下文**：读取 `FEATURE_SPEC` 与 `.specify/memory/constitution.md`，并加载 `IMPL_PLAN` 模板（已复制到目标目录）。

3. **执行计划工作流**：按照 `IMPL_PLAN` 模板结构完成：
   - 填写 Technical Context（未知项标记为 `NEEDS CLARIFICATION`）
   - 依据 constitution 填写 Constitution Check
   - 评估各类 gate（若存在无法合理豁免的违规则报 ERROR）
   - Phase 0：生成 `research.md`（解决所有 `NEEDS CLARIFICATION`）
   - Phase 1：生成 `data-model.md`、`contracts/`、`quickstart.md`
   - Phase 1：运行 agent context 更新脚本
   - 设计完成后重新评估 Constitution Check

4. **停止并汇报**：在完成 Phase 2 规划后结束命令。汇报当前分支、`IMPL_PLAN` 路径与已生成制品。

## 阶段说明

### Phase 0：提纲与调研

1. 从 Technical Context 中提取未知项：
   - 每个 `NEEDS CLARIFICATION` -> 一个调研任务
   - 每个依赖项 -> 最佳实践调研任务
   - 每个集成点 -> 集成模式调研任务

2. 生成并分派调研代理：

```text
For each unknown in Technical Context:
  Task: "Research {unknown} for {feature context}"
For each technology choice:
  Task: "Find best practices for {tech} in {domain}"
```

3. 将结论汇总到 `research.md`，格式：
   - Decision: [选择了什么]
   - Rationale: [为什么选]
   - Alternatives considered: [评估过哪些备选]

**输出**：`research.md`，并确保所有 `NEEDS CLARIFICATION` 已被解决。

### Phase 1：设计与契约

**前置条件**：`research.md` 完成。

1. 从 feature spec 提取实体 -> `data-model.md`：
   - 实体名、字段、关系
   - 源于需求的校验规则
   - 必要时的状态迁移

2. 定义接口契约（若项目有对外接口）-> `/contracts/`：
   - 识别对用户或其他系统暴露的接口
   - 按项目类型写入合适契约格式
   - 示例：库的公共 API、CLI 命令 schema、Web 服务 endpoint、解析器 grammar、UI contract
   - 若项目纯内部（脚本/一次性工具）可跳过

3. **更新 agent 上下文**：
   - 运行 `.specify/scripts/powershell/update-agent-context.ps1 -AgentType codex`
   - 脚本会检测当前 AI agent 类型
   - 更新对应 agent 的上下文文件
   - 仅新增当前 plan 中出现的新技术
   - 保留手工维护区块中的内容

**输出**：`data-model.md`、`/contracts/*`、`quickstart.md`、agent 专属上下文文件。

## 关键规则

- 使用绝对路径
- gate 失败或澄清未完成时必须报 ERROR

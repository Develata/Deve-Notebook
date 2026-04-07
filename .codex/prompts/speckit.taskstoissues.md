---
description: 基于现有设计制品，将任务转换为可执行、按依赖排序的 GitHub Issues。
tools: ['github/github-mcp-server/issue_write']
---

## 用户输入

```text
$ARGUMENTS
```

在继续之前，你**必须**考虑用户输入（若不为空）。

## 大纲

1. 在仓库根目录执行 `pwsh -File .specify/scripts/powershell/check-prerequisites.ps1 -Json -RequireTasks -IncludeTasks`，解析 `FEATURE_DIR`、`TASKS` 与 `AVAILABLE_DOCS`。所有路径必须为绝对路径。若参数含单引号（如 `I'm Groot`），使用 ` 'I'\''m Groot' ` 转义。
1. 从脚本结果中提取 **TASKS** 路径。
1. 运行以下命令读取 Git 远程地址：

```bash
git config --get remote.origin.url
```

> [!CAUTION]
> 仅当 remote 是 GitHub URL 时，才可继续后续步骤。

4. 若 `FEATURE_DIR` 包含 `spec.md`，读取其 Upstream Alignment 节，提取验收条目 ID（如 `STOR-001`、`DIFF-003`）供后续 issue 引用。

5. 对任务列表中的每个任务，使用 GitHub MCP server 在与该 remote 对应的仓库中创建 issue。每个 issue 必须包含：

   **标题格式**：`[Phase N] Task description`

   **Body 结构**：
   ```markdown
   ## Context
   - Feature: [branch name]
   - Phase: [phase number]
   - Dependencies: [task IDs this depends on]

   ## Description
   [task description from tasks.md]

   ## Acceptance Criteria
   [derived from spec requirements + upstream acceptance IDs]

   ## Upstream Traceability
   - 验收条目: [e.g. STOR-001, DIFF-003]
   - 上游文件: [e.g. deve-note plan/04_storage.md]
   ```

   **Labels**：根据任务类型自动添加：
   - Phase 标签：`phase:0-research`、`phase:1-design`、`phase:2-impl`等
   - 类型标签：`type:setup`、`type:test`、`type:core`、`type:integration`、`type:polish`
   - 若任务标记为 `[P]`，添加 `parallel` 标签

   **Milestone**：若仓库已有与 feature branch 同名的 milestone，关联之；否则不创建新 milestone。

> [!CAUTION]
> 在任何情况下，都**不得**在与 remote URL 不匹配的仓库中创建 issue。

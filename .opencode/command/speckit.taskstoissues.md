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

1. 在仓库根目录执行 `.specify/scripts/powershell/check-prerequisites.ps1 -Json -RequireTasks -IncludeTasks`，解析 `FEATURE_DIR` 与 `AVAILABLE_DOCS`。所有路径必须为绝对路径。若参数含单引号（如 `I'm Groot`），使用 ` 'I'\''m Groot' ` 转义。
1. 从脚本结果中提取 **tasks** 路径。
1. 运行以下命令读取 Git 远程地址：

```bash
git config --get remote.origin.url
```

> [!CAUTION]
> 仅当 remote 是 GitHub URL 时，才可继续后续步骤。

1. 对任务列表中的每个任务，使用 GitHub MCP server 在与该 remote 对应的仓库中创建 issue。

> [!CAUTION]
> 在任何情况下，都**不得**在与 remote URL 不匹配的仓库中创建 issue。

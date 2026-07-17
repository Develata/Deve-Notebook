# 01_terminology.md - 功能术语篇

本章描述产品对用户、测试者和操作者公开使用的核心术语语义。

## 功能目标

- 关键对象名称必须稳定，避免同一概念在 UI 中多次改名。
- 用户看到的术语应能对应到稳定心智模型，而不是工程内部细节。

## 功能项

### 1. Repo / Branch / Note

- `Repository / Repo` 表示当前工作仓库。
- `Branch` 表示当前视角处于本地还是远端 peer 分支。
- `Note / Document` 表示可打开与编辑的 Markdown 文档。

### 2. Source Control 术语

- `Changes` 表示工作区未提交变化。
- `Staged Changes` 表示准备提交的变化。
- `History / Graph` 表示已提交版本历史。

### 3. 只读与旁观

- `Read-only` 表示当前不能写入。
- `Spectator / Remote` 表示用户正在查看远端分支，而不是本地可写工作分支。
- `Workspace ingestion unavailable / 工作区摄取不可用` 表示当前服务无法可靠接收该本地 repo 的外部文件变化，因此相关写操作暂时只读；它不表示 Ledger 数据已损坏。
- 普通用户界面使用能力名与恢复提示，不暴露 watcher backend、thread、generation 或路径等实现术语。

## 非目标

- 不要求把内部术语如 ledger、projection、vector 直接暴露为普通用户主术语。
- 不允许同一控件在不同页面对同一概念使用不同名称。

## Chrome MCP 验收实例

### TERM-FEAT-01: 核心术语一致

前置条件：

- 打开应用首页并进入主工作流。

步骤：

1. 查看顶部、侧栏、Source Control 与状态栏。
2. 记录 `Repository / Branch / Changes / History / Read-only` 等术语出现位置。

期望结果：

- 同一概念在主要界面中命名一致。
- 不出现“同物多名”或误导性术语。

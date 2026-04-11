# 06_repository.md - 仓库与分支体验篇

本章描述 repo / branch / spectator 的用户工作流。

## 功能目标

用户应当理解并感知：

- 当前自己正在操作哪个 repo
- 当前自己位于本地还是远端分支
- 何时是可写，何时是只读
- 从远端返回本地时会回到哪个工作上下文

## Operation 示例

- 原子操作示例见 `docs/features/operations/repo_open_doc.md`。
- 该示例将打开文档拆为打开 Quick Open、输入查询、选择文档、请求 OpenDoc、接收正文五个 user operations。
- 仓库切换流示例见 `docs/features/operations/repo_switch.md`。
- 分支切换流示例见 `docs/features/operations/repo_branch_switch.md`。
- 文档结构写操作流示例见 `docs/features/operations/repo_file_operations.md`。

## 功能项

### 1. 仓库切换

- 用户可以切换当前激活 repo。
- Source Control、Explorer、当前文档作用域应随 repo 切换而同步变化。
- 界面不应同时把所有 repo 混成一个全局工作区。

### 2. 本地分支

- 本地分支是默认工作分支。
- 在本地分支下，文档编辑与 Source Control 写操作可用。

### 3. 远端分支 / Spectator

- 切换到远端分支后，用户进入只读旁观者模式。
- 只读状态必须可见，且编辑/提交类操作不能假装成功。

### 4. 返回最近稳定本地 Repo

- 用户从远端切回本地时，应优先回到最近一次稳定使用的本地 repo。
- 如果这个 repo 已不可用，系统才应走 fail-closed 的回退路径。

### 5. 仓库级只读与隔离

- 远端 repo 的只读限制不能影响本地 repo 的正常可写。
- 当前 repo 损坏或失效时，系统应提示恢复或回退，而不是静默绑定到别的 repo。

## 非目标

- 当前阶段不支持任意创建用户自定义“feature branch”。
- 当前阶段不允许在 spectator/remote 视图中直接修改远端 ledger。

## Chrome MCP 验收实例

### REPO-FEAT-01: 本地 Repo 切换

前置条件：

- 至少存在两个本地 repo。

步骤：

1. 在当前 repo 观察 Explorer 和 Source Control。
2. 切换到另一个本地 repo。
3. 观察树、标题、变更列表和当前文档上下文。

期望结果：

- 当前界面完整切换到目标 repo。
- 不残留上一个 repo 的文档与变更状态。

### REPO-FEAT-02: 远端旁观者模式

前置条件：

- 存在至少一个远端分支或 remote scope。

步骤：

1. 切换到远端分支。
2. 尝试进入 Source Control 或编辑器写操作。

期望结果：

- 页面明确显示只读/旁观者状态。
- 写操作不会假装成功。

### REPO-FEAT-03: Remote -> Local 恢复

前置条件：

- 用户已在某个本地 repo 工作过。
- 然后切到远端分支。

步骤：

1. 从远端分支切回本地。
2. 观察最终回到哪个 repo。

期望结果：

- 优先回到最近一次稳定使用的本地 repo。
- 若该 repo 已不可用，系统显式走受控回退而不是静默绑定任意 repo。

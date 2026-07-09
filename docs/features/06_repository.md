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
- SearchBox 文件操作共享壳层见 `docs/features/operations/repo_file_op_shell_routing.md`。
- 文档结构写操作流示例见 `docs/features/operations/repo_file_operations.md`。

## 功能项

### 1. 仓库切换

- 用户可以切换当前激活 repo。
- Source Control、Explorer、当前文档作用域应随 repo 切换而同步变化。
- 界面不应同时把所有 repo 混成一个全局工作区。
- 仓库展开界面应提供新增、重命名与移除本地 repo 的入口：顶部新增按钮用于创建 repo，每个 repo 行的更多菜单用于重命名或移除该 repo。
- 普通移除仓库不应直接销毁 ledger 或 Projection Workspace；用户可见文案必须避免暗示已经物理擦除数据。

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

### 6. Projection Backup

- Projection Backup 是 Remote Projection Transport 的 backup-oriented 产品语义：
  将 Projection Workspace 中的 Markdown files 上传到 WebDAV/S3，或从 remote
  下载 Markdown files 覆盖 Projection Workspace。
- Projection Backup 不备份 ledger history，不传输 encrypted pack、branch manifest、
  RestoreCandidate、snapshot/runtime state、`.git/` 或 `.notegit/`。
- `push` 只上传 Markdown projection files；provider metadata、ETag、mtime、object
  version 与 remote listing order 都只能作为 diagnostics。
- `pull` 只写 Projection Workspace，并必须进入 Watcher/scan -> External Changes；
  用户确认前不得写 ledger、Source Control staging、commit anchor 或 Git mirror queue。
- Web / Command Palette 只提交 provider/direction typed intent；backend 负责解析当前
  repo scope、Projection Locator、Remote Projection locator/profile 与 credential ref。未来
  S3-compatible UX 只能选择 backend-defined profile handle，不得在前端收集 endpoint URL
  或 secret material。
- CLI 使用 `projection-remote webdav push/pull` 与 `projection-remote s3 push/pull`
  执行 Projection Backup transport；旧 `backup` CLI surface 已从首版命令面删除。
- S3-compatible `s3+https://` endpoint 走 host-local、secret-free Remote
  Projection profile binding；CLI 已支持显式 profile handle，未绑定或未匹配 profile
  时仍在 provider I/O 与默认 AWS credential 解析前 fail-closed。

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

### REPO-FEAT-04: 本地 Repo 管理菜单

前置条件：

- 至少存在两个本地 repo。
- 当前处于 local writable scope。

步骤：

1. 展开仓库切换界面。
2. 点击顶部新增按钮，创建一个新 repo。
3. 点击某个 repo 行右侧更多菜单，执行重命名。
4. 点击非当前 repo 行右侧更多菜单，执行移除。

期望结果：

- 新增 repo 后自动切换到新 repo。
- 重命名后列表、标题与当前 scope 显示新名称，`RepoId` 不变。
- 移除后目标 repo 从普通列表消失，ledger authority 与 Projection Workspace 未被物理删除。

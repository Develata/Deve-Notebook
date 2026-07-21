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
- 仓库展开界面应提供新增、设置本地 alias 与移除本地 repo 的入口：顶部新增按钮用于创建 repo，每个 repo 行的更多菜单用于修改当前 host 的显示 alias 或先打开backend removal preview。
- 普通移除仓库采用Prepare → Execute：preview只显示backend生成的preserved/deleted categories与typed warning/blocker，确认后才消费短期token。它会删除本机该repo的Ledger history与Deve-owned runtime state，但保留Projection Workspace容器、Markdown/附件、`.git`与其它非`.notegit`文件。确认文案必须准确说明两类结果，不能写成“所有数据已删除”或“仅从列表隐藏”。
- 最后一个repo可以移除；完成后界面进入`NoScope`空状态，仍可登录、查看诊断和创建首个新repo。没有fallback不应显示为删除失败。
- create/remove 的 repo list 与 scope 结果只在 watcher mount 最终 outcome 已知后更新；页面不得先显示成功再自行补偿。
- create 已提交但 workspace ingestion mount 失败时，新 repo 保留只读可见，当前 session 不自动切换。
- alias 修改只更新当前 host 的列表/标题显示；不停止 watcher、不移动 workspace、不改变可写状态、scope 或其它 peer 的名称。
- alias 缺失时显示完整 RepoId。不同 repo 可以使用相同 alias，但此时按 alias 选择必须要求用户明确 RepoId。
- CLI 支持 deterministic JSON 导出/导入 alias。导入遇到本机不存在的 RepoId、非法 alias、重复 RepoId 或单项失败时 warning + skip，最终完整列出跳过项与原因；其它有效项仍作为一个原子 accepted batch 应用。
- remove 成功后目标 repo 不再启动 watcher；remove 尚未提交即失败时，系统才可以恢复旧 repo 的 watcher。

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

### 6. Remote Projection 与 Remote Import

- Remote Projection 只负责把当前 Markdown Projection Workspace push 到 WebDAV/S3，或把远端内容 streaming 给 project-owned import sink；它不再把 remote 直接 pull/overwrite 到 workspace。
- Remote Import 以不可变 manifest/blob snapshot 建立独立 session。Prepare 完成前不写 workspace；Review 只显示 backend label、Added/Modified/Unchanged、typed blocker 与 typed diff。
- 用户只能整 session Apply 或 Discard；没有 checkbox、逐文件选择和 remote Delete。任一 blocker 禁用整个 Apply。
- Apply 通过 sealed whole-session Ledger transaction提交，事务内先保存“Ledger 已提交、Projection outcome pending”的 durable receipt，随后才执行 Projection writeback。成功收敛为 Written；失败与 durable fault 一起收敛为 Degraded。崩溃/重试从 Ledger 幂等恢复，不重复导入、不回滚 Ledger。
- Refresh 只重算已封存 snapshot：在 RepoId/branch/source/locator 仍 exact 时可把新 revision 绑定到当前 Ledger head 与 ignore snapshot；source/locator/branch/membership/tamper drift 不可重绑。要读取新的远端内容必须先 Discard 后重新 Prepare。
- active session 或 cleanup pending 会阻止 repo remove；用户必须显式 Discard 或运行 dry-run 后的 repair cleanup。alias 修改不搬 RepoId-based artifacts，也不使 session stale。
- Web / Command Palette 只提交 typed intent；backend 解析 repo scope、locator/profile、credential ref、session/revision 与 blockers。S3-compatible UX 只能选择 backend-defined profile handle，不收集 endpoint URL 或 secret。
- 旧 pull→workspace→External Changes 已由 B4 一次删除，不作为兼容能力；backend/CLI Remote Import 已激活，现有 RepoId/provider lifecycle coordination已存在，ownership-aware removal owner-plan由R4补齐，独立Web review surface由B5交付。正式命令面见`14_commands#remote-import-command-contract`。

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
3. 点击某个 repo 行右侧更多菜单，修改本机 alias。
4. 点击非当前 repo 行右侧更多菜单，执行移除。

期望结果：

- 新增 repo 后自动切换到新 repo。
- 上述自动切换仅在新 repo 成功 mounted 时发生；若 durable create 已提交但 mount 失败，新 repo 只读可见且当前 session 保持原 repo。
- alias 修改后列表与标题显示新名称，`RepoId`、scope、writer readiness 与 workspace path 不变；修改非当前 repo 不改变当前 scope。
- Prepare必须显示Remote Import owner返回的typed状态：未Applied的session作为warning并由Execute显式discard/cleanup；Applied/Pending、Applied/Degraded或unknown/corrupt artifact作为blocker。用户不需要在正常可清理状态下先跳到另一个页面手工清空。
- 移除后目标repo从普通列表消失，本机canonical`.redb`、workspace`.notegit`、locator、alias与owner-issued Remote Import capture均已删除；watcher/provider/session slot由各runtime退休而不是作为文件删除。persistent authority lock空文件保留为host协调身份；workspace root、Markdown/附件、`.git`、remote shadow与位于reserved removal roots之外的operator恢复输入保留。
- Prepare发现active operator recovery input与`.notegit`、canonical Redb或Remote Import target重叠时必须阻断。顶层`.notegit`或DB identity replacement不能靠同RepoId文本marker自动repair；child link/reparse entry只删除entry本身且external target保留。
- Web不得显示manifest/path/digest/token binding或raw cleanup detail，也不得自行推断blocker、TTL、fallback或repair动作。confirmation token只驻留当前connection内存并绑定认证session/connection；刷新或重连后必须重新Prepare。

### REPO-FEAT-05: Repo lifecycle 的 mount partial outcome

前置条件：

- 至少存在一个 Mounted local repo。
- 测试入口可分别让 create 后 mount 或 remove 前 final reconcile 失败，并可验证 alias store failure。

步骤：

1. 创建 repo 并让新 watcher mount 失败。
2. 修改另一个 repo alias，并注入 alias store write failure。
3. 对第三个 repo 执行移除并让 final reconcile 失败。

期望结果：

- create 返回“已创建但工作区摄取不可用”，新 repo 只读可见且当前 session 未切换。
- alias store failure 不改变旧 alias；成功 alias set 只更新 display，不发生 remount、scope 切换、Projection Fault 或 Remote Import stale。
- final reconcile 失败发生在 durable remove 前时，remove 不提交并可恢复旧 watcher；任何混合事实都进入 repair，不由 UI 猜测回滚。
- Execute先原子持久化token consumption与job admission，再关闭产品写门、quiesce provider、完成watcher E2，之后才进入authority Quiescing。任一pre-cut失败不得留下“token已消费但job不存在”的不可恢复状态。
- pre-cut失败必须逆序恢复authority、watcher、exact provider generation并释放Transitioning reservation；provider resume或其它补偿失败时repo保持typed readonly/repair，不得伪装成正常Active。
- durable remove已提交但optional fallback在最终publication前失效时，不绑定失效repo、不静默选择第三个repo，也不把成功删除改报为error。发起者与所有仍绑定removed RepoId的observer session各自提交新的`NoScope` epoch并撤销writer-ready；Web只消费backend最终RepoList/lifecycle outcome，清除对应pending remove/scope-switch intent，不丢弃editor pending overlay。
- membership remove的authority cut在短`Catalog -> Repo(target)` lane内原子替换该RepoId的bounded catalog record并轮换process-local generation，且不遍历session；旧binding从cut起立即拒写。session fan-out在permit外执行，前端不得实现旧两帧partial-stage或从`SC_REPO_NOT_SELECTED`推断删除成功。
- repo list/scope publication 只出现最终一次 typed outcome。
- cleanup成功后先fsync durable terminal result并释放authority lock handle，再best-effort投递session/network publication；投递失败不得阻止same-RepoId readmission。

# 16_web_thin_client_ledger.md - Web 写入体验篇

本章描述 Web 端作为 thin client 时，编辑、pending、ack、reject 与写入就绪的用户体验。

## 功能目标

- 用户应知道什么时候可以安全编辑。
- 用户应知道哪些改动只是暂存，哪些已经确认。
- 用户不应被假 pending、错 repo 握手或静默 reject 误导。

## 功能项

### 1. Pending Local Edits

- 本地编辑后，页面可暂时显示未确认状态。
- 这种状态必须可被确认或明确失败。
- 原子操作示例：[`operations/doc_edit_confirmed_op.md`](./operations/doc_edit_confirmed_op.md)

### 2. Ack / Reject

- 服务端确认后，pending 应消失并进入已确认状态。
- 明确 reject 后，pending 不得永久悬挂。

### 3. Repo-Scoped Write Readiness

- 当前 repo 未握手完成时，页面不能假装可写。
- 切 repo、切 branch、重连后，写入状态必须随当前 scope 切换。
- 断线、未授权或 native session 阻塞时必须撤销旧 writer-ready，直到新连接完成当前 scope 的握手与 writer registration。
- 收到命中当前文档的后端 projection recovery 时，旧 document projection-ready generation
  必须立即失效；同一连接/scope 的 repo writer grant 保留，以允许新的 `OpenDoc` 完成恢复。
  检测到 incoming gap 时则撤销 repo writer-ready 并退休连接。页面保留 pending edits，以新
  generation 的 Snapshot/History 恢复；只有 fresh document projection-ready 后才重发一次。
- recovery plan 未命中当前文档时，仅刷新后端指定的列表投影，当前编辑器不得被无谓锁住。
- active repo remove 已提交后，发起connection与仍exact绑定removed RepoId的observer只消费backend单帧typed `RepoRemovalScopeFinalized`；它原子携带final RepoList与RepoBound/NoScope结果。fallback binding失效时发起者进入NoScope而不报删除失败，observer使用各自新scope epoch进入NoScope；已独立切离的observer不受影响。页面不得从列表自动选择其它repo，也不得解析detail或Source Control error决定结果。
- removal Prepare preview、preparation_id与confirmation token只绑定当前authenticated connection epoch并驻留内存；Execute必须使用新的request_id引用exact preparation_id。断线、刷新或scope mismatch立即丢弃未消费token，不写入URL、browser storage或telemetry，也不在前端计算TTL/blocker。
- finalization必须在一次state update中应用RepoList与scope、关闭旧writer-ready并保留editor pending overlay。F4/v5不存在旧`RepoList -> SC_REPO_NOT_SELECTED`两帧stage、10秒partial timeout或旧epoch第二帧。

### 3.1 Remote Import Thin-Client State

- Remote Import 是独立 review surface，不属于 editor pending、External Changes 或 Source Control。
- 页面只显示后端返回的 immutable session state、candidate revision、backend-generated label、typed
  change kind/blocker 与 typed diff；不会显示 locator、host/provider/blob path、digest、credential 或
  raw failure detail。
- Prepare/List 的 pending request 只绑定 `request_id + repo/branch/scope`；`Prepared` 通过该 gate 后才安装
  backend 生成的 session/revision，`Listed` 的每个 summary 自带其 session/revision。Show/Page/Diff/
  Refresh/Apply/Discard 等 session-bound response 必须再与当前 session/revision 精确匹配。
- repo、branch、scope 或已选择的 session/revision 任一变化后，不匹配的迟到 response 都会被丢弃；
  页面不会把旧 Remote Import state 迁移到新 scope。
- Refresh 只重算已封存 snapshot；Apply/Discard 作用于整个 session。首版没有 checkbox 或逐文件选择。
- Prepare 可以在当前 repo 暂时未 Mounted 时完成 review；Apply 是否可用只由后端 typed blocker
  决定。Apply receipt 的 Projection outcome 为 Pending 时，页面显示 Ledger 已提交且恢复未完成；为
  Degraded 时显示已提交与降级提示。两者都不会诱导重复 Apply。
- B4 已激活 backend typed family 并删除命令打开 Source Control 的路径；独立 client/view 在 B5
  实现，期间不以 Source Control 或 External Changes controller 代替。

### 4. 离开文档保护

- 如果当前文档仍有未确认本地写入，离开前必须明确提示。
- 但被明确 reject 的写入不应继续卡成“等待确认”。
- 原子操作示例：[`operations/doc_pending_navigation_guard.md`](./operations/doc_pending_navigation_guard.md)

## 非目标

- 当前阶段不把 Web 端视为完整离线 authority。
- 当前阶段不允许 UI 仅凭快照加载完成就认为当前可写。

## Chrome MCP 验收实例

### WEBWRITE-FEAT-00: Projection Recovery 保留 Pending

前置条件：两个客户端打开同一 repo，客户端 A 当前文档存在 pending overlay。

步骤：在客户端 B 通过 External Changes 应用一次命中 A 当前文档的批量修改；随后注入一次
浏览器 incoming gap，并恢复网络。

期望结果：A 进入只读 `Resyncing`，pending 不丢失；只处理 fresh generation 的
Snapshot/History，writer-ready 恢复后 pending 只重发一次。对无关文档执行相同 Apply 时，A 的
当前编辑器保持可写。若 adapter/replay 再次失败，页面进入可诊断 Error 并只提供显式 Retry。

### WEBWRITE-FEAT-01: Pending -> Ack

前置条件：

- 打开一个可写文档。

步骤：

1. 输入少量文本。
2. 观察 pending 状态。
3. 等待服务端确认。

期望结果：

- pending 短暂出现。
- 确认后回到已就绪状态。

### WEBWRITE-FEAT-02: Reject 不会永久悬挂

前置条件：

- 构造一次明确的 reject / persist fail 场景。

步骤：

1. 编辑文档。
2. 观察错误与 pending 状态。
3. 尝试离开页面。

期望结果：

- 页面明确说明写入失败。
- 不会永远卡在“等待服务端确认”。

### WEBWRITE-FEAT-03: Repo Scope 切换后的写入边界

前置条件：

- 存在本地与远端或多个 repo scope。

步骤：

1. 在当前 repo 进入可写状态。
2. 切换到只读或未就绪 scope。
3. 再切回健康可写 repo。

期望结果：

- 只读 scope 下不能假装可写。
- 回到健康 repo 后，写入状态与当前 scope 精确一致。

### WEBNAV-FEAT-01: Pending 编辑离开提示

前置条件：

- 当前文档存在未确认本地编辑。

步骤：

1. 尝试打开其他文档、切 repo、切 branch 或返回 Home。

期望结果：

- 页面显示离开保护提示。
- 原本的 pending 编辑仍保留。

### WEBNAV-FEAT-02: Pending 编辑离开取消

前置条件：

- 离开保护提示已显示。

步骤：

1. 选择留在当前文档。

期望结果：

- 当前文档仍可见。
- pending 编辑未被丢弃。

### WEBNAV-FEAT-03: Pending 编辑确认离开

前置条件：

- 离开保护提示已显示。

步骤：

1. 明确选择继续离开。

期望结果：

- 提示关闭。
- 原先保存的导航动作执行。

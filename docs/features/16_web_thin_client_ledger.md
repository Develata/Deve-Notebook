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

### 4. 离开文档保护

- 如果当前文档仍有未确认本地写入，离开前必须明确提示。
- 但被明确 reject 的写入不应继续卡成“等待确认”。
- 原子操作示例：[`operations/doc_pending_navigation_guard.md`](./operations/doc_pending_navigation_guard.md)

## 非目标

- 当前阶段不把 Web 端视为完整离线 authority。
- 当前阶段不允许 UI 仅凭快照加载完成就认为当前可写。

## Chrome MCP 验收实例

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

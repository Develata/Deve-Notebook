# 04_storage.md - 存储与恢复体验篇

本章描述用户可感知的保存、恢复、重建与存储异常行为。

## 功能目标

- 用户需要知道“什么时候改动真正保存成功”。
- 用户需要知道工作区异常时系统会如何恢复。
- 用户不应被误导为“文件看起来变了就等于已确认写入”。

## 功能项

### 1. 已确认保存

- 文档写入只有在系统确认后才算真正保存成功。
- 已确认写入在重新打开文档或刷新页面后仍应存在。
- 原子操作示例：[`operations/doc_edit_confirmed_op.md`](./operations/doc_edit_confirmed_op.md)

### 2. 工作区偏差

- 外部编辑器或文件系统导致的偏差，应以工作区变更形式出现。
- 用户应能区分“工作区有变化”和“已经提交到权威状态”。
- `.deveignore` 命中的外部路径不应由 watcher/scan 加入工作区变更列表，即使它们在 watcher 启动前已存在或由目录事件触发了扫描。
- 外部整目录删除与原子替换完成后，External Changes 应自动刷新到最终 pending 状态；
  这类刷新不代表变更已经写入 Ledger。

### 3. 异常恢复

- projection、workspace 或局部缓存异常时，系统应优先从权威状态重建。
- 用户不应因局部故障而看到静默丢失的数据。

### 4. 损坏与降级

- 某个 repo 损坏时，应进入明确的降级或恢复路径。
- 其它健康 repo 不应被一起拖死。
- Projection Workspace 路径若包含绝对路径、遍历段、空段、Windows 非规范尾随点/空格、内部 `.git` / `.notegit` 段，或已有符号链接 / junction 指向该 repo workspace 外部，保存、重建与 materialize 必须明确失败；系统不得在 workspace 外创建、覆盖或删除文件，Ledger 中已确认的 authority facts 保持可用于 repair。
- 缺失 ledger entry 格式信封、缺失 redb schema version 或 schema version 不匹配时，系统应明确报告该 repo 需要 reset / repair / migration，不应猜测旧格式继续打开。
- 对明确选择的 schema v2 开发库，用户可以在服务停止时运行 `deve export --allow-legacy-v2` 做只读 JSON/Markdown 导出；该入口不会打开正常写入、同步或 repair authority。
- workspace ingestion failure 与 repo/projection 损坏是两类不同状态。前者只使当前进程无法可靠接收该 repo 的外部文件变化：该 repo 保持只读可见，Ledger inspect/export 与诊断仍可用，不能被显示为 `DegradedProjection`。
- 多 repo server 中，一个 repo 的 workspace ingestion failure 不应关闭其它已 mounted repo；健康 repo 继续可写。服务启动时若没有任何 repo 成功 mounted，则明确启动失败。
- 运行期 workspace ingestion failure 的首版恢复方式是重启服务；产品不提供 watcher restart endpoint，也不要求用户理解 backend/thread 细节。
- owned watcher 关闭时先停止并 join backend producer；已开始的 dispatch 完成后丢弃尚未消费的 hint，再按启动时的 exact RepoId 与 canonical root 做一次 final full reconcile，并最多发送一次 typed refresh。关闭返回后不得再产生 callback 或 pending candidate；stop/final-scan 失败只作为 typed primary/cleanup 诊断，不覆盖既有 worker 首因。

### 5. 工作区摄取健康与写入阻断

- `/api/node/role` 可以公开 `healthy / transitioning / degraded / unknown` 的 aggregate workspace ingestion health，以及 expected/running/unavailable 数量。
- aggregate 不显示 repo 名、RepoId、路径、generation 或失败原因；具体失败详情只用于 operator logs。
- 依赖当前 workspace 状态的 editor、Docs、External Changes、Source Control、merge、Remote Import Apply 与 plugin writer，在当前 repo 未 mounted 时统一返回本地化的“工作区变更暂时不可用”；Remote Import Prepare/Review 不写 workspace，不受 mount gate 阻断。
- Ledger 已提交但 Projection/workspace writeback 失败时，该 repo 跨重启保持
  `DegradedProjection`；只有后端完成精确 repair 并清除 active fault 后才恢复。
  历史 Remote Import `Degraded` receipt 不会因后续 repair 被改写为 `Written`。
- 纯读、Ledger inspect/export、remote shadow ingest 与 offline repair/export/diagnostic 不受该 blocker 影响。
- 前端只渲染后端 typed code 与 aggregate health，不解析自然语言 detail，也不自行判断何时重启或恢复写权限。

## 非目标

- 当前阶段不把 Projection Workspace 当作唯一权威真相。
- 当前阶段不允许未确认写入被显示层静默“算作成功”。

## Chrome MCP 验收实例

### STORAGE-FEAT-01: 已确认写入可复现

前置条件：

- 打开一个可写文档。

步骤：

1. 输入一段新文本。
2. 等待状态恢复到已确认。
3. 刷新页面并重新打开文档。

期望结果：

- 刷新后内容仍然存在。
- 不出现“看起来写了但实际没存”的情况。

### STORAGE-FEAT-02: 未确认写入不会被误判为成功

前置条件：

- 制造一次写入失败或显式 reject 场景。

步骤：

1. 编辑文档。
2. 观察 pending / error 状态。
3. 尝试离开当前文档。

期望结果：

- 系统明确说明该写入未确认。
- 不会把失败写入继续显示为“还在等待成功”。

### STORAGE-FEAT-03: 损坏 Repo 隔离

前置条件：

- 存在一个损坏 repo 和一个健康 repo。
- 损坏 repo 可表现为无版本 ledger entry 或无 redb schema version 的开发期旧库。

步骤：

1. 启动应用。
2. 观察默认进入的 repo 与文件树。
3. 切换到健康 repo 并打开文档。

期望结果：

- 健康 repo 正常可用。
- 损坏 repo 不会把整个应用拖入不可用状态。

### STORAGE-FEAT-04: 忽略文件不经 watcher 进入工作区变更

前置条件：

- repo 的 Projection Workspace 根目录包含 `.deveignore`，规则匹配该 repo 下的 Markdown 路径。

步骤：

1. 在 watcher 启动前创建一个被忽略的 Markdown 文件。
2. 启动应用或 watcher。
3. 再创建另一个同样被忽略的 Markdown 文件。
4. 打开 Source Control / 工作区变更列表。

期望结果：

- 两个 ignored 文件都不出现在 pending / staged / committed 变化中。
- 文件树不把 ignored 文件当作可同步笔记。

### STORAGE-FEAT-05: 外部目录删除刷新 External Changes

前置条件：

- repo 中存在一个包含多个已确认 Markdown 的目录；External Changes 面板已打开。

步骤：

1. 在外部文件管理器或终端中删除该目录。
2. 等待 watcher debounce 完成。
3. 观察当前 repo 的 External Changes 变更列表。

期望结果：

- 目录内已跟踪 Markdown 自动显示为待确认删除，无需另一个文件事件触发刷新。
- Ledger 在用户显式 Apply / Commit 前不增加对应删除事实。
- 临时 access/open 目录事件不会产生虚假的 External Changes 刷新。

### STORAGE-FEAT-06: Repo-local workspace ingestion 故障隔离

前置条件：

- server 同时管理两个健康 local repo，其中 Repo A 的 watcher 可注入 terminal failure，Repo B 保持 Mounted。

步骤：

1. 在 Repo A 触发 watcher failure，并尝试编辑、创建文档和执行 External Changes mutation。
2. 在 Repo A 打开文档、读取历史或执行只读诊断。
3. 切换到 Repo B，编辑并确认一项变更。

期望结果：

- Repo A 的新 workspace-dependent mutation 统一显示“工作区变更暂时不可用”，且没有副作用；只读能力仍可使用。
- Repo B 继续正常可写，不被 Repo A 故障拖死。
- UI 不展示 Repo A 的路径、generation 或 watcher failure detail，也不把它标成 projection 损坏。

补充 host/runtime 验收：

- bootstrap 完成后若零个 repo Mounted，server 必须清理已启动 handle 并非零退出；不能以“全局只读但已就绪”继续启动。
- supervisor invariant、generation corruption、thread/resource exhaustion 或 runtime coordination failure 等 typed host-fatal 必须全量回滚已启动 watcher 并终止 server；不得通过字符串匹配扩大或缩小 host-fatal 集合。
- server 已运行后即使所有 watcher 后续均 Failed，也必须保留只读与诊断入口，并把 aggregate ingestion health 投影为 degraded；不得因 repo-local terminal failure 退出整个进程。

### STORAGE-FEAT-07: Aggregate ingestion health 是薄前端投影

前置条件：

- `/api/node/role` 返回至少一个 unavailable repo 的 aggregate health。

步骤：

1. 打开应用并观察 workspace ingestion blocker/health surface。
2. 捕获 `/api/node/role` 响应。
3. 检查页面 console 与 network response。

期望结果：

- 页面按 typed status/code 显示本地化只读提示与“重启服务”恢复指引。
- node-role 只包含 status/expected/running/unavailable aggregate，不泄漏 repo identity、路径或失败详情。
- 前端没有解析 detail、调用 restart endpoint 或执行业务恢复判断。

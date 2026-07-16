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

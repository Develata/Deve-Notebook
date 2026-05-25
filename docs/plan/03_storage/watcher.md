# 03_storage/watcher.md - Watcher Runtime

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-05-24`
- `Parent`: `03_storage/index`
- `Primary Code Areas`: `crates/core/src/sync/watcher/`, `crates/core/src/watcher.rs`, `crates/core/src/watcher_ignore.rs`, `crates/core/src/writeback/suppressor.rs`

> 本文件是 `03_storage` 章的 `watcher_runtime` 子合同：外部编辑生命周期、watcher ingestion 路径与 watcher contract。章节骨架与总览见 [index.md](./index.md)。

## 5. State Machines（watcher 部分）

> §5.1 Repo Mount Lifecycle 见 [index.md](./index.md)；§5.2 Write Lifecycle 见 [authority.md](./authority.md)。

### 5.3 External Edit Lifecycle

```text
FsEvent
  -> Debounced
  -> PendingFsRecorded
  -> Staged
  -> LedgerCommitted
  -> Cleared
```

补充：

- `FsEvent`
  - source: notify backend / FileObserver / kqueue equivalent
- `Debounced`
  - duplicate burst events coalesce by path + final content/inode state
- `PendingFsRecorded`
  - effect: side table row inserted/updated, never authority mutation
- `Staged`
  - effect: explicit user confirmation only
- `Cleared`
  - effect: consumed by commit or discarded by reset-to-projection

## 6. Ledger-First Write Paths（watcher 部分）

> §6.1 Path A 与 §6.3 Path C 见 [authority.md](./authority.md)。

### 6.2 Path B: Watcher / External Edit Ingestion

1. watcher 捕获文件系统事件。
2. 经 debounce、路径归一化、`.deveignore` / internal path 过滤、inode 解析后写入 `pending_fs_ops`。
3. 非文档目录事件只允许触发 repo-scoped scan；scan 必须复用同一套忽略规则。
4. 仅暴露 working directory 偏差，不改变 authority。

规则：

- watcher 事件 **MUST NOT** 直接写 ledger。
- delete / rename / move 必须先成为候选，再经 Stage / Commit 进入结构事实。
- 被忽略路径 **MUST NOT** 通过 watcher/scan 反向摄入到 `pending_fs_ops`、tree projection 或 ledger。

## 8. Watcher Contract {#watcher-contract}

### 8.1 Backend Abstraction

- 必须存在统一 `FsWatcherBackend` trait。
- Desktop / Android / iOS 后端必须在后端层归一化事件语义。

### 8.2 Startup Semantics

- watcher_start 是 repo open 的最后一步。
- 启动前必须执行一次全量 scan。
- 启动扫描 **MUST** 读取 repo workspace root 下的 `.deveignore`，并在创建 pending candidate 前跳过被忽略的 Markdown。
- scan 与 watcher 首批事件之间的去重必须由 side table 幂等性保证。

### 8.3 忽略与路径过滤

- `.deveignore` 位于 repo workspace root；直接 watcher 事件、目录重扫与启动扫描 **MUST** 使用同一套 repo-relative 匹配语义。
- 忽略匹配 **MUST** 接受 repo-relative path（`<path>`）；不再存在 vault-wide ignore 语义。
- `.notegit/` 与其它 repo 内部目录 **MUST** 按路径段语义忽略；`.notegit-backup` 这类同名前缀兄弟路径 **MUST NOT** 被误判为内部目录。
- 被忽略 Markdown **MUST NOT** 通过 watcher/scan 生成 `Added`、`Modified`、`Deleted` 或 rename pending entry，也 **MUST NOT** 在 scan 中被当作 tracked doc 缺失处理。

### 8.4 Self-Write Suppression

- projection/persist_doc/commit apply 写盘前必须向 repo-local `WriteSuppressor` 注册写回指纹。
- watcher 在匹配窗口内必须丢弃自写事件。
- suppressor 状态必须 repo-local，禁止全局共享。

### 8.5 Overflow Recovery

- queue overflow / dropped events 时，watcher **MUST** 触发全量 reconcile。
- reconcile 完成前 **MUST** 暂停继续消费增量事件。

### 8.6 Lifecycle

- repo close / switch **MUST** 停止对应 watcher 并 drain 事件。
- 同一 repo **MUST NOT** 同时存在多个 watcher。

### 8.7 Debounce and Atomic Write Semantics

- debounce window **SHOULD** 为 `50ms-200ms`
- debounce window **MUST NOT** 为 `0`
- atomic write / temp-file replace 必须统一收敛成单次 pending modify / rename candidate
- rename pair 识别失败时，宁可退化为 pending delete + pending create，也不得伪造 authority rename

## 10. Forbidden Patterns（watcher）

> watcher 无独立专属禁止项；“未经 Stage / Commit 让 watcher 事件直接入 ledger” 归 [authority.md](./authority.md)，跨层禁止项见 [index.md](./index.md)。

## 11. Runtime Boundary（watcher 部分）

### 11.3 Watcher Layer

- 负责外部文件事件归一化、忽略规则、debounce、self-write suppression 与 overflow reconcile。
- watcher 只能生成 pending candidate，不得直接写 ledger。

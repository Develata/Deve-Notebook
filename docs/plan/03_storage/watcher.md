# 03_storage/watcher.md - Watcher Runtime

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-08-26`
- `Parent`: `03_storage/index`
- `Primary Code Areas`: `crates/core/src/sync/watcher/`, `crates/core/src/watcher_ignore.rs`, `crates/core/src/writeback/suppressor.rs`, `apps/cli/src/watcher_runtime.rs`, `apps/cli/src/server/runtime/watcher_runtime.rs`

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

### 8.1 Backend Adapter and Bounded Capture

- 必须存在统一 `FsWatcherBackend` adapter。Desktop / Android / iOS backend 必须在 adapter 内把平台事件归一化为 project-owned `FsEventHint` / `BackendSignal`；`notify::DebouncedEvent`、平台 FFI type 与 backend-specific error **MUST NOT** 越过该边界。
- `BackendSignal` 必须区分可归一化 hint、需要 full reconcile 的 signal 与 terminal receive failure；backend error、rescan flag、queue overflow、oversized batch、跨 root rename 歧义均不得被猜测为完整增量事实。
- Running 阶段使用非阻塞有界队列，固定最多 `16` 个 batch。每个 batch 最多 `256` 个 `FsEventHint`，全部路径 UTF-8 载荷合计最多 `256 KiB`；任一界限触发时不得阻塞 backend callback，也不得提交部分 batch，只设置 level-triggered reconcile latch。
- CaptureOnly 或 full reconcile 期间到达的 raw event 只用于置 dirty latch；不得缓存、排序或 replay raw event。
- read/open/access 等非变更目录事件 **MUST NOT** 触发 repo scan，也不得占用目录刷新 cooldown；未知变更类型必须置 reconcile latch。

### 8.2 Capture-First Startup Cut (A3′)

watcher start 是 repo open 的最后一步，且必须按以下顺序执行：

```text
Starting(generation)
  -> prepare/canonicalize workspace root
  -> attach backend in CaptureOnly
  -> full scan
  -> atomic compare-exchange CaptureOnly(clean pass token) -> Running
  -> CAS success: Running
  -> dirty: retry full scan
```

- CaptureOnly mode、dirty latch、scan-pass token 与 `TerminalFailure` 必须由同一个原子状态（或具有同等线性化证明的单次 CAS）承载。每个 pass 在 full scan 开始前建立新的 clean token；backend callback 必须通过同一原子状态三选一：recoverable error/rescan 或普通 hint 在 Running cut 前改变 token/置 dirty，使 handoff CAS 失败；cut 后普通 hint 进入有界队列（满时置 reconcile latch）；terminal receive failure/panic 在任意 cut 前原子写入 `TerminalFailure`。所有 Running handoff CAS 遇到 terminal state 必须失败并返回原始首因，禁止把它重试/改写为 `StartupChurn`，也不得发布已退出 backend 的 Running handle。禁止“先 load clean、再独立 store Running”，不得存在既未污染 scan pass 又未进入 Running queue 的 callback 窗口。
- 只有一个 clean scan pass 的 exact token CAS 成功，才能把 handle 发布为 `Running`；scan 期间发生的任何 hint、backend rescan/error signal 或 `.deveignore` 变化都必须保持 dirty，并触发下一次 full scan。
- 启动最多执行三个 scan pass。第三次仍 dirty 时必须返回 typed `StartupChurn`，停止并 join backend/worker，且不得发布半初始化 handle。
- 生产 runtime 不设置固定 30 秒或其它 wall-clock scan deadline，避免合法大 repo 被误杀；测试与 release producer 必须从宿主侧设置硬超时并报告 timeout evidence。
- prepare、attach、scan、consumer spawn、Running handoff 任一步失败都必须停止 backend，并在 `WatcherFailure` 中同时保留 primary 与 cleanup 原因；cleanup 失败不得覆盖 primary。
- generation 必须在所有 worker completion 与状态写回处 exact-compare；旧 generation 不得修改或关闭新实例。
- 启动扫描必须读取 workspace root 下的 `.deveignore`，并在创建 pending candidate 前跳过被忽略的 Markdown。scan 与后续增量 hint 的重复候选由 side table 幂等性消解。

### 8.3 Ignore and Path Filtering

- `.deveignore` 位于 repo workspace root；直接 watcher hint、目录重扫、启动 scan 与 final reconcile **MUST** 使用同一套 repo-relative 匹配语义。
- `.deveignore` 文件本身的 create/modify/remove/rename 必须在任何语义过滤之前置 dirty latch，并要求 full reconcile；不得用旧 ignore matcher 先过滤该变化。
- 忽略匹配 **MUST** 接受 repo-relative path（`<path>`）；不再存在 vault-wide ignore 语义。
- `.notegit/` 与其它 repo 内部目录 **MUST** 按路径段语义忽略；`.notegit-backup` 这类同名前缀兄弟路径 **MUST NOT** 被误判为内部目录。
- 被忽略 Markdown **MUST NOT** 通过 watcher/scan 生成 `Added`、`Modified`、`Deleted` 或 rename pending entry，也 **MUST NOT** 在 scan 中被当作 tracked doc 缺失处理。

### 8.4 Self-Write Suppression

- projection/persist_doc/commit apply 写盘前必须向 repo-local `WriteSuppressor` 注册写回指纹。
- watcher 在匹配窗口内必须丢弃自写事件；若 suppressor 无法证明事件属于自写，必须按外部变化处理或置 reconcile latch，不得静默丢弃。
- suppressor 状态必须 repo-local，禁止全局共享。
- suppressor 的共享状态锁不得覆盖文件读取、存在性检查或内容 hash；验证必须使用带 generation 的短锁 claim，
  文件系统 I/O 在锁外执行，settlement 只能消费仍与该 generation 相同的登记，不能误删并发的新登记。
- 过期登记一旦被同 path claim、同 repo insert 或周期 GC 识别，必须退休；显式 clear 或最后一次命中移除
  path 后，空 repo bucket 必须同步退休。suppressor 不得因历史 repo/path 数量无界保留空状态。

### 8.5 Level-Triggered Reconcile

- queue full、oversized batch、dropped event、backend recoverable error/rescan、跨 root rename 歧义与 CaptureOnly/reconcile 期间的变化必须收敛到同一个 level-triggered reconcile latch。
- latch 为 set 时，consumer 必须停止增量 dispatch，丢弃尚未处理的 hint batch，并执行 full reconcile。
- latch **MUST** 只在 full reconcile 成功且原子确认 reconcile 期间没有再次置 dirty 后清除；scan 失败或 dirty 再次出现时不得提前 clear。
- Windows backend 的 kernel overflow 只有在受控 dependency source 能把 zero-byte completion / `ERROR_NOTIFY_ENUM_DIR` 传播为 rescan signal，且 producer receipt 证明 reconcile 后事实完整时，才算满足本条。依赖升级优先；无合格稳定版本时只允许经单独外部授权的最小 pinned patch。

#### 8.5.1 Windows Overflow Source and Producer Contract (C2→C1)

- 实施 W9 时必须先复核是否已有包含等价 overflow→rescan 修复的官方稳定 `notify` 版本；若有，直接升级官方版本并运行同一 producer。若没有，才允许准备 C2 最小 patch、测试、registry 与 upstream PR 文案，并在创建/push fork 或提交 PR 前暂停等待外部授权。
- C2 patch 只能修改 upstream `notify/src/windows.rs`：把 zero-byte completion / `ERROR_NOTIFY_ENUM_DIR` 转成 `Flag::Rescan`，且必须先 rearm OS watcher 再发布 rescan。不得改变公开 API、buffer size、其它平台代码或无关 Windows 行为。
- `[patch.crates-io]`、`Cargo.lock` 与 dependency source override 只有在取得可解析的真实 40 位 revision 后才可提交；禁止假 SHA、临时本地路径或不可重现 source。官方稳定版本满足同等行为后必须整体删除 override，不保留双轨或兼容分支。
- Windows producer 必须由独立进程连续执行三次 `callback barrier + 2048 file burst`，每次同时证明：overflow 传播为 Rescan；rearm 后仍收到正常事件；成功 full reconcile 后 pending 集合与 producer 独立计算的 expected hash 完全一致。
- receipt 必须绑定 dependency source/revision、Windows build、filesystem、当前精确 HEAD 与三次独立进程结果。`STORE-016` 在该 receipt 存在前保持 required gap，不得用普通 unit test 或 source-ref 代替 overflow 真实性。
- `v0.1.0 Public Preview` 可以在 `18_release` 的 exact-version typed freeze 中把该
  required gap 登记为 accepted known limitation，但不得改写矩阵 evidence kind、生成伪
  receipt、关闭 STORE-016 或声称 watcher 已完整收敛。CHANGELOG 与 Release notes 必须明确
  说明数千外部文件事件风暴的影响、重启规避和官方稳定 notify family + 三进程 receipt 的退出条件。

### 8.6 Owned Handle and Host Supervision

core 提供不可 `Clone` 的单 repo owner：

```rust
RepoWatcherHandle::start(RepoWatcherStart)
    -> Result<RepoWatcherHandle, WatcherStartError>
RepoWatcherHandle::repo_id() -> RepoId
RepoWatcherHandle::generation() -> u64
RepoWatcherHandle::snapshot() -> RepoWatcherSnapshot
RepoWatcherHandle::shutdown(self) -> Result<(), WatcherFailure>
RepoWatcherHandle::shutdown_bounded(self, timeout) -> Result<(), WatcherFailure>
```

- `RepoWatcherHandle` 唯一拥有该 repo 的 backend、consumer worker、stop/join 边界与 typed snapshot。正常路径必须显式消费 handle 调用 `shutdown()` 并处理结果；`Drop` 只允许 best-effort 安全网，不能作为正常 lifecycle 或错误报告路径。
- 独立命令可以使用完整 `shutdown()` 收敛；进程级 server termination 必须使用 `shutdown_bounded()`，先发出同一 shutdown 请求，再只在调用方剩余总截止时间内等待 worker join。截止时间耗尽时必须消费 handle、退休 command sender，并把仍在等待的 join 移交给不阻塞 Tokio runtime/process exit 的 detached join waiter；返回固定 typed shutdown failure，不能因 `Drop` 再进入无界 join，也不能把 timeout 伪装为成功清理。
- `RepoWatcherSnapshot` 至少携带 `repo_id / generation / worker_state`；公开状态类型固定为 `RepoWatcherWorkerState::{Running, Failed(WatcherFailure)}`。`WatcherFailure { phase, kind, primary, cleanup }` 中的 `primary` 永远保留首因，`cleanup` 只附加 stop/final-scan/join 诊断。
- prepare/attach/scan/consumer-spawn/Running-handoff 的所有 start failure 必须由 `WatcherStartError` lossless 携带同一个 `WatcherFailure`，禁止丢弃 cleanup 或从字符串重建 taxonomy；supervisor 在调用 core start 前产生的 reservation/busy/invariant error 属于独立 host error，不得伪装为 backend `WatcherFailure`。
- project-owned `WatcherRefresh` 是 core 到 host 的唯一刷新 callback；host 可把它映射到现有 repo-scoped `FsChangeDetected`，但 core 不依赖 WS message、UI 状态或 server sender。
- CLI server host runtime 的 `WatcherSupervisor` 是多 repo slot、generation、start/stop/restart 与故障隔离的唯一所有者。standalone `deve watch` 直接拥有其 handle 集合并在 terminal failure 时逆序关闭。`WatcherRuntimeView` 只允许 `AppState`、mutation admission 与 `/api/node/role` 读取 snapshot/aggregate；handler 不得获得 start/stop/restart 权限。
- 不保留全局 watcher registry、按 `RepoId` 停止的 free function、全局 running probe 或返回 `RepoId` 的旧 start 形态；此前开发 API 未发布，不提供 deprecated adapter、双轨或兼容分支。
- 同一 repo 同一时刻只能有一个 supervisor slot。slot reservation 与 generation 分配必须先于 backend/thread 创建，并在失败时精确释放；supervisor map mutex 不得跨 filesystem I/O、scan、join、await、mutation lane 或 publication。

handle worker lifecycle：

```text
Starting(generation)
  -> Running -> Stopping -> Stopped | Failed
       |           ^
       v           |
     Failed -------+
```

- receive、dispatch、reconcile error 或 worker panic 必须以 generation guard 原子转为 `Failed`；首发不自动重启，operator recovery 为重启服务。
- failure transition 与 mounted admission check 必须在线性化的 slot state 上完成。failure cut 之前已获准的 operation 可以完成；切点之后的新 workspace-dependent mutation 必须 fail-closed。

### 8.7 Debounce and Atomic Write Semantics

- debounce window **SHOULD** 为 `50ms-200ms`。
- debounce window **MUST NOT** 为 `0`。
- atomic write / temp-file replace 必须统一收敛成单次 pending modify / rename candidate。
- rename pair 识别失败时，宁可退化为 pending delete + pending create，也不得伪造 authority rename；跨 root paired rename 必须置 reconcile latch，不得整体忽略。
- 目录删除事件即使在消费时目标路径已经不存在，也 **MUST** 依据 backend remove kind 触发 repo-scoped scan；不得用 post-event `metadata(NotFound)` 将其静默过滤。
- 目录删除 scan **MUST NOT** 被通用目录 cooldown 丢弃；同一 debounced batch 内的多个删除目录事件 **MAY** 合并为一次 repo scan。
- atomic temp-file replace 的 pending hash **MUST** 对应最终目标文件内容；临时文件 **MUST NOT** 单独进入 `pending_fs_ops`。
- 成功完成目录 scan 或 full reconcile 后，watcher **MUST** 经 `WatcherRefresh` 请求至多一次 repo-scoped `dir_changed` refresh；消费端据此重新读取 External Changes 与 tree projection，该消息本身不改变 authority。server 的 generation-bound refresh routing 由 supervisor slot 承载：slot 为 `Transitioning` 时只能 coalesce/defer，不能直接 broadcast；最终 lifecycle outcome 后由 `RepoLifecycleCoordinator` 决定 enqueue 或 drop。standalone `deve watch` 可直接消费 refresh。
- Remote Import Prepare/Show/Page/Diff/Refresh/Discard 只操作 host-only sealed
  session state，不是 Projection Workspace input，watcher不得观察、扫描或把
  它们投影成 External Changes。只有 Remote Import Ledger Apply 提交后的正常
  projection writeback会沿既有 PersistGuard/WriteSuppressor 路径更新 workspace。

### 8.8 Final-State Shutdown (E2)

`drain` 唯一定义是 final-state reconcile，不是 raw event replay：

```text
Stopping
  -> stop backend production and join debouncer
  -> signal consumer Stopping and await dispatch-quiesced barrier
  -> discard queued hints
  -> final full reconcile
  -> optional one WatcherRefresh
  -> join consumer
  -> Stopped | Failed
```

- dispatch-quiesced barrier 必须证明 consumer 已停止领取新 batch，且任何已开始的 dispatch/pending write 已完成；barrier 之后 consumer 只能等待 finalization/exit，不得再次 callback 或写 pending。queued hints 只能在该 barrier 后整体丢弃，final reconcile 不得与 consumer dispatch 并发。
- final reconcile 必须发生在 handle ownership 仍有效且 repo workspace/locator 尚可读取时。成功后若 pending/tree projection 可能变化，最多由 shutdown coordinator 发送一次 refresh；发送后再解除 consumer 等待并 join。
- final scan 失败不妨碍 backend/debouncer/consumer cleanup；`shutdown()` 必须返回失败，并同时保留 primary 与 cleanup 原因。
- worker failure epilogue 必须复用相同 cleanup 顺序；原始 receive/dispatch/reconcile/panic 原因不得被 stop、final scan 或 join error 覆盖。
- `shutdown()` 返回后不得再 callback、写 pending candidate 或保留可运行 thread。重复停止只能通过 supervisor 对已终止 slot 的 typed 状态处理；不可复制 handle 后重复 shutdown。

### 8.9 Multi-Repo Readiness and Failure Isolation

- `RepoMountState` 是 process-local readiness，与 `04_repository#repo-health-and-repair` 的 `RepoHealth` 正交。在线 workspace-dependent local write 的固定条件为 `RepoHealth::Healthy && RepoMountState::Mounted`。
- repo-local start/worker/final-reconcile failure 只把该 repo slot 标记为 `Failed`；其它 mounted repo 继续运行。零个 local repo 时 server 以 healthy `NoScope` 启动；存在 local repo 但最终零个 `Mounted` 时仍保留只读、诊断与 Create 能力。只有 typed supervisor/runtime host-fatal 才必须逆序关闭已启动 handle 并退出。
- server 启动后即使全部 watcher 失败，仍保留纯读、ledger inspect/export、remote-shadow ingest 与 offline repair/export/diagnostic；不得把 server 整体伪装成健康可写。
- host-fatal 只允许 typed 分类：supervisor invariant、generation corruption、thread/resource exhaustion、runtime coordination failure。`runtime coordination failure` 仅表示 process-global coordination state 已无法证明或继续安全运行，不得作为 repo-local backend/scan/dispatch/cleanup error 的兜底分类。host-fatal 必须回滚全部 handle；不得按错误字符串决定 host-wide shutdown。
- 默认 scope、browser writer registration 与所有 workspace-dependent mutation admission 必须排除非 `Mounted` repo；repo list 可继续显示失败 repo 的只读可恢复状态。

### 8.10 Public Diagnostics Boundary

- workspace ingestion unavailable 的唯一产品错误码由 [`13_i18n#error-code-catalog`](../13_i18n.md#i18n-error-code-catalog) 定义；HTTP mutation 返回 JSON `ServerError` + `503`，editor WS 复用 `EditRejected`，其它 WS mutation 复用 `ProtocolError`。不新增 watcher lifecycle WS message。
- `/api/node/role` 只暴露 [`07_network`](../07_network.md) 定义的 aggregate `watcher_health`；不得暴露 repo 名、`RepoId`、generation、路径、failure phase 或 detail。
- 失败详情只进入结构化 tracing。产品 `detail` 使用固定泛化文本；Web 只根据 typed code/i18n 与 aggregate health 渲染 blocker，不解析 detail，不决定 restart，也不推断 repo 是否可写。

### 8.11 Dynamic Repo Lifecycle Deferral

- create/remove 与 watcher mount 的 transaction、partial outcome、锁序和 publication 唯一由 [`04_repository#repo-lifecycle-coordinator`](../04_repository.md#repo-lifecycle-coordinator) 定义。
- host-local alias 修改永不进入 watcher lifecycle。watcher layer 只提供 generation-bound start/shutdown/snapshot；不得自行修改 catalog membership、locator、alias 或 active session。

### 8.12 Watcher Convergence Seal (W10)

- W10 的 non-overflow seal 必须在同一精确 HEAD 封存 capture-first startup cut、typed Failed、E2 final-state shutdown、dynamic create/remove、server repo isolation 与 standalone fail-all。
- Windows/Linux real-filesystem receipts 与 create/remove、repo-isolation browser receipts 必须绑定该同一 HEAD；alias independence 由独立 host-local runtime producer证明，不能冒充 watcher evidence。旧 HEAD、source-only、手工叙述或不同 artifact set 的结果不得拼接签署。该 non-overflow seal 关闭 STORE-007，但不能代替 STORE-016。
- 只有 W9 overflow producer 和全部 watcher 合同检查通过后，runtime registry 的 `watcher_runtime` 才可改为 `已收敛`。`v0.1.0 Public Preview` 使用 exact accepted gap 时必须保持 `部分承载` 与 STORE-016 gap；后续版本在退出条件满足前仍不得复用该 accepted gap。

## 10. Forbidden Patterns（watcher）

- 未经 Stage / Apply 让 watcher candidate 直接进入 Ledger。
- 让 backend-specific event/error type 越过 `FsWatcherBackend` adapter。
- 用无界 channel、阻塞 backend callback、缓存/replay raw event 或在 overflow 后继续消费不完整增量后缀。
- 用全局 registry/free function 共享 watcher ownership，或让 handler/UI 获得 watcher lifecycle authority。
- 把 watcher `Failed` 写成 projection fault、`DegradedProjection`、同步事实或用户数据。
- 用字符串分类 host-fatal，或由前端解析 error detail 推断写权限/恢复动作。

## 11. Runtime Boundary（watcher 部分）

### 11.3 Watcher Layer

```text
UI / HTTP / WS handlers
  -> typed intent / RepoLifecycleJobRuntime
  -> RepoLifecycleCoordinator
  -> RepoMutationPublicationGate + WatcherRuntimeView
  -> WatcherSupervisor
  -> RepoWatcherHandle
  -> FsWatcherBackend adapter
  -> notify/platform backend
```

- watcher layer 负责外部文件 hint 归一化、忽略规则、debounce、self-write suppression、bounded capture、full reconcile 与 owned lifecycle。
- watcher 只能生成 pending candidate 与 typed refresh，不得直接写 Ledger、决定 writeback、修改 repo catalog/locator 或持有 UI/session state。
- `WatcherSupervisor` 属于 host runtime composition；`RepoWatcherHandle` 与 backend adapter 属于 core execution domain；二者通过 project-owned typed API 连接，不得反向依赖 handler 或 transport。

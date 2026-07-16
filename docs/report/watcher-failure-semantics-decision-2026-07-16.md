# Watcher failure semantics decision report — 2026-07-16

> 本报告是 `docs/report/` 下的时点证据，不是 live contract。下列停止点尚未获得
> USER 批准；除“已实施的小型修复”外，本报告不授权修改 watcher authority、启动顺序、
> 失败状态或恢复策略。

## Metadata

- `Date`: 2026-07-16
- `Status`: `USER decision required`
- `Authority contract`: `docs/plan/03_storage/watcher.md`
- `Scope`: watcher startup, lifecycle, failure recovery, Windows overflow
- `Out of scope`: ledger/write authority、wire/storage format、前端业务判断

## 1. Current state and invariants

Watcher 只把外部文件系统事实归一化为 `pending_fs_ops` candidate；它不直接写 Ledger，
也不拥有 writeback 决策。现有正确边界应保持：

```text
filesystem event -> watcher normalize/reconcile -> pending_fs_ops
                                            -> explicit confirmation -> Ledger
```

本轮已在既有合同内完成五项小型修复，不依赖本报告裁定：

1. 已删除目录不再因消费时 `metadata(NotFound)` 被静默过滤，而是触发 repo-scoped scan；
2. 非变更 `Access/Open` 目录事件不再抢占 refresh cooldown；删除目录绕过通用 cooldown，
   同一 debounced batch 只执行一次 forced scan；
3. notify backend stop 显式调用 `Debouncer::stop()`，并在正常 stop 与 consumer error
   两类退出路径等待底层事件线程；cleanup error 不覆盖 primary error；
4. 目录/full rescan 成功后经既有 `FsChangeDetected(dir_changed)` typed callback 通知
   当前 repo 的 External Changes 与 tree projection 重新读取，不改变 authority；
5. atomic replacement、目录整树删除、stop 后无新 candidate 获得 Windows/Linux 本地
   real-FS 验证，并已配置 Ubuntu/Windows CI matrix；CI receipt 仍须由 exact HEAD 产生。

以下问题会改变启动顺序、失败状态或恢复所有权，已停止在决策层，没有实现。

## 2. Evidence

### 2.1 Startup blind window

`start_repo_watcher` 当前顺序为：

```text
precheck registry -> initial scan -> canonicalize -> attach backend -> spawn consumer -> register
```

`initial scan` 完成后到 backend watch 建立前存在窗口。窗口内发生且之后不再变化的文件系统
事件，既不在 initial scan 中，也不保证由 backend 收到。

### 2.2 Worker failure is not represented

`run_loop` 对 backend receive、dispatch 和 rescan 错误使用 `?` 退出。registry 只有
`Running` / `Stopping`；worker 自行退出不会移除或转换 slot。因此可能出现：

- event consumer 已停止；
- `is_repo_watcher_running` 仍返回 true；
- 没有 typed health/error state、重启策略或上层降级信号。

### 2.3 Windows overflow evidence chain is incomplete

锁定依赖是 `notify 8.2.0` 与 `notify-debouncer-full 0.7.0`。对锁定源码的复审显示，
Windows `ReadDirectoryChangesW` completion callback 的 `_bytes_written` 未参与判断，
未知错误路径记录日志后 unwatch，未向本项目提供可被归一化为 `Rescan` 的事件。因而本项目
“收到 backend error/rescan flag 后同步 reconcile”的代码成立，但无法证明 Windows kernel
overflow 一定能抵达该入口。

### 2.4 Lifecycle transaction gaps

- duplicate start 在 registry precheck 与最终 insert 之间仍可短暂创建两个 OS watcher；
  后插入者会被拒绝并停止，但 `Starting` 尚不是 registry 状态。
- CLI/server 批量启动多个 repo 时，后续 repo 启动失败不会自动停止本轮已启动的前序 watcher。
- 跨 repo root 的 paired rename 当前整体忽略；不同 OS/backend 是否将 move-out 表示为
  paired event 尚缺双平台实证。
- stop 当前先观察 stop signal，再停止 backend；已经进入 backend/debouncer 队列但尚未交付给
  consumer 的事件不会被显式 drain。合同中的 `drain` 究竟表示“处理完队列”还是“关闭并明确
  丢弃未交付事件”尚未定义。

### 2.5 Safe-fix verification receipt

本轮已实施的小型修复获得以下本地证据；它们不替代候选 HEAD 的 CI receipt：

- Windows 与 WSL/Linux real-FS fixtures 均通过 atomic replacement、目录整树删除、stop 后
  无新 candidate、writeback suppression 与 rename pairing；
- `deve_core` library tests 597 项通过，watcher 定向测试 29 项通过；
- Chrome MCP 连接真实 `deve_cli serve --dev --port 3001` 与 Trunk 页面，在已打开的
  **External Changes** 面板中删除含 `alpha.md` / `beta.md` 的已跟踪目录后，无需第二个
  文件事件即自动显示两条待确认删除；浏览器捕获到 repo-scoped `FsChangeDetected`
  及后续 `/api/sc/staged`、`/api/sc/pending` 200 响应，console 无 warning/error；
- 同时执行 `deve_cli sc status --repo default` 得到
  `staged=0 unstaged=2 confirmed=0`，证明删除仍停留在 pending side table；
- Web 定向测试 `fs_change_refreshes_external_changes_sibling_view` 证明正确 scope 会调用既有
  typed External Changes facade，旧 `scope_nonce` 不会触发刷新。Source Control 继续只展示
  confirmed ledger changes，两者未合并。

## 3. Decision A — startup ordering

### Options

| Option | Design | Benefit | Cost / failure mode |
| --- | --- | --- | --- |
| A0 | 保持现状 | 零变更 | 保留不可观测漏事件窗口 |
| A1 | initial scan -> attach backend -> second idempotent scan -> consumer | 在不缓存 raw event 的情况下覆盖窗口；复用现有 scan | 改变启动顺序；第二次 scan 增加启动 I/O；scan 失败时需关闭 backend |
| A2 | attach backend 并缓存 event -> scan -> replay | 可严格定义 event cut | 需要队列上限、排序、overflow 和 replay 去重，复杂度最高 |

**建议裁定 A1。** second scan 仍只生成 pending candidate，不改变 authority；backend 必须在
第二次 scan 失败时同步 stop。若必须证明严格 event cut，再单独设计 A2，不在首发前引入。

## 4. Decision B — worker failure state and recovery owner

### Options

| Option | Design | Benefit | Cost / failure mode |
| --- | --- | --- | --- |
| B0 | 保持 `Running` 假象 | 零实现成本 | 外部变更可静默停止摄取，最危险 |
| B1 | registry 增加 typed `Failed { cause }`；上层显式观测；首版不自动重启 | 失败可见、恢复所有权明确、避免重启风暴 | 需要状态模型、查询/诊断 API、server/CLI shutdown 处理 |
| B2 | worker 内自动重启 + backoff | 短暂错误可自愈 | 可能无限重启、丢失窗口、状态复杂，仍需 health state |
| B3 | 任一 watcher 失败即终止 server | 最简单的 fail-closed | 多 repo 可用性差，单 repo 故障放大为全服务退出 |

**建议裁定 B1。** `Failed` 只表达 watcher ingestion health，不把 repo/ledger authority 标为损坏；
恢复由显式 stop/restart 或更高层 operator 动作触发。首发不做隐式自动重启。是否把 repo 对外
呈现为 degraded、是否阻止其他写路径，以及是否新增 public crate API 或 wire/operator
endpoint，需要在 plan 中明确，不能由 crate-internal typed helper 代替该裁定。

## 5. Decision C — Windows overflow recovery

### Options

| Option | Design | Benefit | Cost / failure mode |
| --- | --- | --- | --- |
| C0 | 保持锁定依赖 | 零变更 | overflow 后可能静默失去 watcher，合同不可被实证 |
| C1 | 升级到已验证会传播 overflow/rescan 的上游版本 | 维护成本最低 | 必须先以锁定源码与 Windows stress receipt 证明；升级可能带 event 语义变化 |
| C2 | pin/fork 最小补丁，明确把 zero-byte/overflow 转为 rescan | 行为可控、可写回归测试 | 维护 fork；需审查 Windows FFI 与 upstream 合并路径 |
| C3 | 周期性 full reconcile 作为安全网 | 即使 backend 静默也最终收敛 | 增加 VPS I/O；不是即时恢复；周期和并发需新合同 |
| C4 | Windows 改用 polling backend | 语义易证明 | 延迟与 I/O 成本最高，平台行为分叉 |

**建议顺序为 C1（若可证明）否则 C2；C3 仅作 defense-in-depth，不替代可观测 overflow。**
在依赖调查和 stress receipt 完成前，不宣称 Windows overflow gate 已满足。

## 6. Decision D — lifecycle transaction

建议与 A/B 一并裁定：

1. registry 增加 `Starting` reservation，在创建 backend/thread 前原子占位；初始化失败释放；
2. CLI/server 的多 repo start 使用局部 guard，任一失败即逆序停止本轮已启动 watcher；
3. 不把跨 root rename 猜测性并入本次修改；先由 Windows/Linux real-FS test 确定 backend
   事件形态，再判断是 delete+create 降级 bug 还是非问题。

收益是消除短暂重复 watcher 和部分启动泄漏；代价是扩展 lifecycle 状态与 rollback 路径。
失败模式包括 reservation 未释放、rollback 二次失败遮蔽首因，因此错误必须保留 primary cause
并附 cleanup diagnostics。

## 7. Decision E — stop/drain semantics

| Option | Design | Benefit | Cost / failure mode |
| --- | --- | --- | --- |
| E0 | stop signal 后关闭 backend，明确丢弃尚未交付 batch | 关闭有界、实现简单 | stop 临界区的外部变更需下次 startup scan 才发现 |
| E1 | 停止 backend 产出后，drain 已交付 batch，再退出 consumer | 尽量保留已捕获事件 | 必须定义 drain 上限、错误处理与 shutdown latency |
| E2 | stop 时强制 final full scan，不回放 raw batch | 最终状态导向、无需事件排序 | 增加关闭 I/O；scan 失败时需定义 repo close 结果 |

**建议裁定 E2。** watcher 本来就是状态重建器而非 event log；final scan 与 side-table 幂等模型
一致。但它改变 stop/write ordering 和失败语义，未获 USER 批准前不实施。

## 8. Impact, migration, and rollback

若 USER 批准建议方案：

- `docs/plan/03_storage/watcher.md` 先定义 startup cut、`Starting/Running/Failed/Stopping`
  生命周期、失败可见性与恢复所有者；
- `docs/features/04_storage.md` 与 acceptance cases 再投影 operator-visible 行为；
- 代码影响限定在 `crates/core/src/sync/watcher/` 与 CLI/server watcher bootstrap，
  不改变 storage format。A/D/E 预计只需 crate-internal API；B 若增加 public crate API、
  wire message 或 operator endpoint，必须在批准后的 plan 中逐项列出；
- A、B/D、C、E 分成可独立回退的 commit；每个 commit 回退后仍回到当前已验证行为；
- C 若涉及依赖升级或 fork，`Cargo.lock` 与依赖来源必须同 commit 回退。

## 9. Verification required after approval

- deterministic fake-backend tests：startup gap、rescan barrier、worker failure、状态转换、rollback；
- Windows/Linux real-FS tests：create/modify/delete、directory removal、atomic replacement、rename、
  stop/restart；
- Windows overflow stress receipt：必须证明 overflow 被转换为 rescan 且 reconcile 后事实完整；
- targeted core/CLI tests、workspace clippy、plan/docs/code coverage 与 storage baseline；
- failure injection 必须证明 cleanup error 不覆盖 primary error；
- 若新增 operator-visible health，或影响现有 Source Control refresh，必须执行 Chrome MCP。

## 10. Consequences of no change

- A0：首次扫描与 watch attach 之间仍可能漏掉一次性变更；
- B0：worker 可停止但 registry 继续报告 running，operator 无可靠诊断；
- C0：Windows overflow/rescan 合同缺少可到达性证明，不能作为首发已闭环能力签署；
- D0：并发 start 仍会短暂创建多 watcher，批量启动失败可能留下部分运行实例。
- E0：stop 临界区的未交付事件只能依靠下一次 startup scan 收敛，当前 `drain` 合同仍不可签署。

这些问题不证明 Ledger authority 已损坏，也不否定本轮已修复的普通目录删除/stop 缺陷；但它们
阻止 watcher 专项复审被签署为“全部失败路径已闭环”。

## 11. Requested USER decision

请分别裁定：

1. `A1`：attach 后 second scan；
2. `B1`：显式 `Failed`，首版不自动重启；
3. `C1 -> C2`：优先验证上游升级，无合格版本时使用最小 pinned patch；
4. `D`：`Starting` reservation + 批量启动 rollback；跨 root rename 只先补实证。
5. `E2`：stop backend 后执行 final full scan，以最终状态而非 raw event queue 定义 drain。

未获明确批准前，以上五项均保持未实现。

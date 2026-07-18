# W7 Repo Lifecycle architecture stop — 2026-07-18

> 本报告是 `docs/report/` 下的时点证据，不是 live contract。W7 在三路只读
> review 后触发架构停止点；USER 已批准 A1 + B1 + C1′，live contract 以
> `docs/plan/04_repository.md` 与相关章节为准。

## Metadata

- `Date`: 2026-07-18
- `Status`: `USER approved A1 + B1 + C1′; implementation in progress`
- `Baseline`: `main@3025b3365531fc81ef2ce92b226840e40b575880`
- `Authority contracts`: `docs/plan/03_storage/authority.md`,
  `docs/plan/03_storage/watcher.md`, `docs/plan/04_repository.md`,
  `docs/plan/07_network.md`, `docs/plan/09_web_thin_client_ledger.md`
- `Scope`: dynamic create/rename/remove ownership, lifecycle linearization,
  committed publication, RepoNameBinding authority and wire admission
- `Explicitly unchanged without further approval`: `DEVELDG3`, Ledger payload v3,
  Projection format, sync fact semantics, Remote Import whole-session transaction

## 0. Projection Fault route A disposition

USER 已批准 Projection Fault 路线 A。该裁定与已接受的 ADR 0012 一致：

- Projection Fault 继续是 repo-local Redb v4 recovery evidence；
- `projection_persistence_runtime` 独占 typed store；
- `RepoHealth` 与 process-local `RepoMountState` 正交；
- lifecycle target/fallback 可以使用 backend-owned typed
  `RepoHealth::Healthy + RepoMountState::Mounted` admission；
- watcher failure 不得创建 Projection Fault，也不得伪装成
  `DegradedProjection`。

现有提交 `afa5f5dbc` 已落地 repo-local Projection Fault store。此次批准允许 W7
补齐 health admission，但不自动批准 rename authority、WS payload、lifecycle owner 或
Catalog/Repo I/O linearization 的变化。

## 1. Current state and blocking evidence

### 1.1 Transport owns a durable lifecycle future

`handle_rename_repo` / `handle_remove_repo` 在 WebSocket handler 内直接 await
`RepoLifecycleCoordinator`，handler 随后才执行 observer fan-out 和 initiator
publication。连接 retire、transport shutdown 或 task cancellation 可以在任一 await
丢弃 future；已进入 `spawn_blocking` 的 E2/filesystem work 不会随之停止，但没有 owner
继续收敛 reservation、committed outcome 或 publication。

Failure modes：

- mount slot 永久停在 `Transitioning`；
- durable mutation 已提交，但 observer 仍持有旧 RepoBound；
- caller 收不到结果时，系统无法区分“未提交”与“已提交、publication 丢失”；
- cleanup/finalization 依赖 handler 生命周期，违反 transport-independent authority。

### 1.2 Core lifecycle mixes prepare, durable cut and settlement

当前 `RepoManager::rename_local_repo` 在单次调用中混合：selector/read、workspace
`rename`、locator write、RepoInfo write、`.notegit` marker write。remove 同时执行 listing、
removed marker 和 locator cleanup；create 同时创建目录、Redb、marker、locator 与 catalog
可见状态。这些调用又位于 `Catalog -> Repo` publication gate 内。

这使慢盘或目录操作扩大为全局 Catalog lane stall，也使 crash/partial truth 无法绑定到明确
的 prepare/cut/settle 阶段。把现有函数机械地移到 permit 外同样不安全，因为 exact
revalidation 与 durable authority cut 会失去线性化点。

### 1.3 Committed outcome is not immutable

remove 完成 removed marker + membership generation cut 后，
`finalize_removed()` 或 final repo-list projection 仍可用 `?` 返回普通 error。handler
因此拿不到 outcome，也不会消费合同要求的 immutable
`RepoRemovalPublicationPlan`。rename 的 list projection/fan-out 也有同类问题。

一旦 durable cut 已完成，后续错误只能是 `CommittedPartial/RepairRequired`，不能重新
伪装成 pre-commit failure。

### 1.4 Repo rename has an authority-model drift, not only a missing field

Live plan 要求：

```text
RenameRepo { repo_id, old_name, new_name, expected_name_epoch }
RepoNameBinding { repo_id, repo_name, name_epoch, changed_at_seq }
```

当前 wire 只有 `repo_id + name + switch_nonce`；current Redb `RepoInfo` 只有
`uuid + name + url`。`rename_local_repo` 直接改写 RepoInfo 并移动 workspace，没有
durable `name_epoch`、stale-intent CAS 或 ledger-derived rename history。

因此不能只在 WS 上补一个 `expected_name_epoch`：backend 没有可 exact-compare 的
authority。若继续现状，会把可变 metadata side table 冒充 plan 中的 RepoNameBinding。

### 1.5 Other contract-local blockers

以下问题不需要新的 architecture decision，但必须在本报告裁定后的 W7 中一起修复：

- provider quiesce 必须早于 watcher E2，且 provider acquire 要在 slot mutex 内 exact
  revalidate `CatalogMembershipToken`；
- rename/remove target 必须 exact `Healthy + Mounted` 且持有 current-generation handle；
- fallback 初选、pre-commit、publication apply 三个切点都要 exact revalidate health、
  membership 与 mount generation；
- RepoBound registry insert 与 membership revoke 必须消除 revalidate→insert 窗口；
- inbound flow 必须先消费已排队 session invalidation，再尝试更新 binding；
- successful `RepoList -> RepoSwitched` 和 observer
  `RepoList -> SC_REPO_NOT_SELECTED` 两种 Web 两帧路径都要收敛；
- partial truth 必须按 RepoId 重读 catalog、RepoInfo/metadata、locator、old/new workspace
  marker、removed marker 与 Remote Import state。

## 2. Decision A — lifecycle operation owner

| Route | Design | Benefit | Cost / failure mode |
| --- | --- | --- | --- |
| A1 | 新增 host-owned `RepoLifecycleJobRuntime`。handler 只提交 typed intent并观察结果；job 不随 transport cancellation 取消。authority cut 产出 immutable committed-cut plan，settlement 再形成 immutable settled publication，由 host-owned sink/session runtime 消费。 | authority 与 transport 解耦；operation 必有 owner；committed publication 可重试；薄 handler | 增加 durable request receipt、single-flight、bounded completion、shutdown/join 与 cancellation 测试 |
| A2 | handler 继续拥有 future，仅用 drop guard / abort guard 补偿 | 改动小 | async cleanup 无法可靠在 Drop 完成；durable commit 后仍可能漏 publication |
| A3 | 把 lifecycle、session map 与 network fan-out合并到一个 actor | 顺序直观 | authority/runtime/network 高耦合；actor 成为大锁和单点故障 |

**建议：A1。**

边界固定为：

```text
handler -> typed lifecycle intent -> RepoLifecycleJobRuntime
                                  -> RepoLifecycleCoordinator
                                  -> immutable committed-cut plan
                                  -> settlement
                                  -> immutable settled publication
                                  -> RepoLifecyclePublicationSink / RepoSessionRuntime
handler <- typed observed result
```

`RepoLifecycleJobRuntime` 不拥有 Ledger/Redb facts，不读取 connection map，不发送网络帧；
它只保证 operation 在 host 进程内有稳定 owner。host-local receipt 支持 request 去重与结果重取；
进程崩溃后不继续执行“job”，而是由 receipt + 完整 partial-truth recovery 判断 durable state，
避免把 lifecycle receipt 升格为 Ledger/业务 workflow authority。

## 3. Decision B — prepare / cut / settle boundary

| Route | Design | Benefit | Cost / failure mode |
| --- | --- | --- | --- |
| B1 | 按 operation 定义 project-owned `Prepared*Lifecycle`、短 authority cut 与 typed settlement。长时 scan/join/workspace I/O在 permits 外；cut 内只做 exact revalidation和最小 durable mutation；cut 立即产出 immutable publication plan。 | 最符合 §7.9；失败阶段明确；可做 crash/partial truth测试 | 需要拆分 current core API，并为 create/rename/remove分别定义 recovery truth |
| B2 | 用专用 blocking actor 串行执行整个 lifecycle，并在 actor 内跨 I/O持有逻辑 Catalog lane | 不阻塞 Tokio worker | 仍把慢盘放大全局串行 stall；隐藏而非消除大临界区 |
| B3 | 保持现有 gate 包裹复合 core 函数 | 实现成本最低 | 已证实违反合同；不可签署 |

**建议：B1。**

固定阶段：

```text
admit + reserve
  -> release permits
  -> provider quiesce / watcher E2 / bounded filesystem prepare
  -> reacquire Catalog -> Repo
  -> exact revalidation + minimal durable authority cut
  -> immutable publication plan
  -> release permits
  -> filesystem settlement / mount finalization / session publication
```

规则：

- pre-cut failure 尝试全部补偿，保留 `primary + cleanup[]`，旧 scope只在 old truth 唯一时恢复；
- post-cut failure永远返回 committed partial，publication plan不可丢；
- `spawn_blocking` panic/JoinError 必须仍能归还 reservation；
- supervisor/session/catalog mutex 不跨 scan、join、filesystem I/O、await 或 network send；
- cut 内若现有 removed-marker/locator representation无法形成最小原子 mutation，应再次停止，
  不得通过双写或猜测回滚掩盖。

## 4. Decision C — immutable identity and host-local alias

USER 批准 **C1′**：repo name 不再是 ledger/sync authority，也不参与 workspace 物理绑定。

- `RepoId` 是不可变跨宿主 logical identity；full peer 只以 RepoId + genesis/ledger identity +
  authenticated source 互认，不传输 alias。
- 当前人类显示名改为 host-local `HostRepoAliasBinding { repo_id, alias, alias_revision }`；
  alias 缺失时显示完整 RepoId。
- Projection Locator 改为持有 immutable `workspace_segment`；本机 create 可以保留
  `<safe_initial_alias>--<repo_id>`，其它首次绑定默认 `<repo_id>`，后续 alias 修改不移动目录。
- 普通 “rename repo” 只做 host-local alias CAS，不经过 watcher、Ledger、sync、Remote Import
  或 Projection Fault。
- JSON v1 import/export 只含 RepoId 与 alias。unknown local RepoId、invalid alias、duplicate
  RepoId 或 per-entry admission failure 均 warning + skip，结尾逐项汇总；valid entries 作为一个
  atomic accepted batch 提交，store-wide commit failure 是全局错误。

该路线最大化多端数据处理系统与本地人类交互系统的独立性，同时保留稳定 RepoId、严格
identity admission、typed CAS 与确定性批量迁移；它不需要改变 Ledger payload、sync fact 或
Projection format。

## 5. Combined recommendation

USER 最终批准：

```text
A1 + B1 + C1′
```

该组合拒绝 handler-owned durable future、跨长时 I/O 的大临界区和 mutable RepoInfo 伪
authority；alias 不再给 authority/data plane 增加跨端负担。

## 6. Impact surface

- `apps/cli/src/server/runtime/repo_lifecycle_runtime*`
- `apps/cli/src/server/runtime/repo_session_runtime.rs`
- `apps/cli/src/server/runtime/watcher_runtime/*`
- `apps/cli/src/remote_import_runtime/provider_tasks.rs`
- `apps/cli/src/server/handlers/switcher/*`
- `crates/core/src/ledger/manager/repo_lifecycle.rs`
- C1′: host-local alias store/CLI、F4/v4 Repo Control client/server messages、Web repo list
- contract projections: repository/storage/network/thin-client/features/acceptance/registry/overview

不允许把 lifecycle判断搬到 Web；Web仍只渲染 typed state、保存 backend identity/epoch、发送
typed intent。

## 7. Migration and rollback

- 尚无正式发布，不保留旧 lifecycle API、WS adapter、双写或 mutable RepoInfo rename分支；
- A1/B1 分成 docs contract、runtime ownership、operation slicing、publication、test evidence等
  rollback-friendly commits；
- 每个 pre-cut API 替换在同一 commit 删除旧复合调用面；
- C1′ 单独提交 alias/locator contract、store/CLI、product surface 与 evidence；rollback 必须整块
  回退至未发布基线，不保留 RepoNameBinding、repo_name_hint 或 workspace-move 双轨。

## 8. Verification after approval

- deterministic barriers：provider acquire/revoke、binding insert/revoke、handler cancellation、
  pre-cut/cut/post-cut每个取消点；
- failure injection：E2、filesystem prepare、durable cut、finalize、repo-list projection、session
  publication，验证 primary/cleanup与 committed partial；
- Failed/Stopped/no-handle target reject；degraded-health fallback三切点 reject；
- Windows/Linux real-FS create/remove 与 alias set/import/export；
- Web成功 fallback、invalid fallback、observer带冲突 pending intent、同名不同 RepoId；
- C1′：JSON negative fixtures、stale alias revision、restart、unknown/invalid warning summary、
  inter-peer alias non-transmission 与 alias/workspace independence；
- full core/CLI/Web tests、fmt/clippy、WASM、plan coverage、Markdown links、storage baseline、
  architecture diff；
- 真实 backend Chrome MCP覆盖最终保留的 repo lifecycle surface。

## 9. Consequences of no change

- transport cancellation可留下无 owner的 Transitioning或已提交未发布状态；
- Catalog lane可能被慢 filesystem I/O占用，单 repo操作放大全局停顿；
- removed membership已撤销但客户端仍持有旧 scope；
- Failed/no-handle repo可绕过 E2执行 rename/remove；
- repo alias继续被伪装成 synced/current RepoInfo 或 workspace authority，无法声称 plan/docs/code严格双射；
- W7、B6、W10和最终 tag-ready均不能签署。

## 10. USER decision

2026-07-18，USER 明确批准 `A1 + B1 + C1′`，并批准 JSON v1 的 warning + skip + final
summary 语义。任何 Ledger envelope/payload、sync fact、Projection format 或本报告之外的
runtime/crate 边界变化仍须再次停止等待批准。

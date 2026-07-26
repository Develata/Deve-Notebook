# 04_repository.md - 仓库与分支工程蓝图

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.2`
- `Last Review`: `2026-07-22`
- `Counterpart Feature`: `docs/features/06_repository.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/07_storage_repo.md`
- `Primary Code Areas`: `crates/core/src/tree/`, `crates/core/src/ledger/manager/structure_projection*.rs`, `apps/cli/src/server/handlers/switcher*.rs`, `apps/web/src/hooks/use_core/callbacks_switch.rs`, `apps/web/src/hooks/use_core/callbacks_switch/`

## 1. Scope

本章定义 Repo、Branch、Tree Projection 与 Repo Health 的工程实现合同。

本章回答的问题只有两个：

1. 系统如何唯一识别并切换到某个 repo / branch / doc scope。
2. 结构树、仓库目录与 repo repair 如何在 authority 与 projection 之间分层。

用户可见行为、按钮文案与 Chrome MCP 手工路径属于 `docs/features/06_repository.md`，不属于本章。

## 2. Authoritative Entities

### 2.1 Repo Identity

- `RepoId` 是仓库不可变的跨宿主权威身份，UUID-first。
- full peer 互认 logical repo 时只交换并核对 `RepoId` 与既有 genesis / ledger identity；不得同步 repo alias。RepoId 相同仍必须验证 authenticated source，不得把碰撞概率低误当作授权。
- `HostRepoAlias` 是绑定到 `RepoId` 的可变 host-local 显示别名，不是身份、同步事实或 workspace authority。
- `URL`、`Selector` 只允许作为输入别名或恢复线索。
- 任何进入业务算子的 repo 输入，在执行前 **MUST** 解析为 `RepoId`。
- 所有 repo instance admission 都必须反向验证 `ledger.repo_id == resolved RepoId`；任何 name/path/url 与 ledger identity 不一致的情况都必须 fail-closed。

### 2.1.1 Host-local Repo Alias Contract {#host-repo-alias-contract}

`HostRepoAlias` 与 `RepoId` 的绑定必须显式建模：

```text
HostRepoAliasBinding = {
  repo_id,
  alias,
  alias_revision,
}
```

约束：

- `host_repo_alias_runtime` 唯一拥有 host-local alias store；Ledger、sync、Remote Import、provider transport、Projection Locator 与 repo catalog 不得保存“当前 alias”的第二份可写副本。
- composition root 内的 alias admission 必须复用该 `RepoManager` 的 process-local `CatalogMembershipRuntime` 快照，避免绕回 durable catalog 文件锁并与 watcher/lifecycle 自竞争；仅无 composition root 的 offline `open_existing` 路径可以读取受跨进程锁保护的 durable catalog snapshot。
- alias 缺失时 display fallback 是完整 canonical `RepoId`，其 CAS revision 固定为 `0`；首次成功写入产生 revision `1`，后续内容变化逐一递增，同值写入是保留原 revision 的幂等成功。alias 可以重复，按 alias 选择命中多个 RepoId 时必须 fail-closed。
- backend repo list/show projection 必须同时返回 `repo_id + display_alias + alias_revision`；Web/CLI 只能回送该 opaque revision，不能自行递增或从 alias 内容推导 revision。
- alias 修改是 host-local CAS：`SetHostRepoAlias { repo_id, alias, expected_alias_revision }`。它不得 append Ledger、改变 repo metadata/genesis、停止 watcher、移动 workspace、更新 locator、使 Remote Import stale 或制造 Projection Fault。
- online alias CAS 必须在短 `Catalog -> Repo(repo_id)` mutation lane 内完成，并在同一 lane 内拒绝 `RepoReadiness::Transitioning`；它仍不获得 watcher 启停或 lifecycle authority。这保证 removal 关闭产品写门后不再有 alias revision 漂移，而 Mounted/Unavailable 只读状态本身不禁止 host-local alias 管理。
- alias 校验只服务人类显示：trim 后非空、UTF-8 最多 256 bytes、不得含 control character 或 NUL。`/`、`\\`、标点与 emoji 可以存在，因为 alias 永不进入物理路径。
- inter-peer sync、Remote Projection transport 与 Remote Import manifest/receipt **MUST NOT** 传输 alias。浏览器控制协议可以接收当前 host 生成的 display alias，但不得把它回写为跨宿主事实。

JSON import/export schema 固定为：

```json
{
  "format": "deve.host-repo-aliases",
  "version": 1,
  "aliases": [
    { "repo_id": "550e8400-e29b-41d4-a716-446655440000", "alias": "math" }
  ]
}
```

- export 只输出 alias store 中的 explicit binding；revision `0` 的 canonical RepoId fallback 不写入 JSON。结果必须按完整 RepoId 排序并使用 deterministic JSON；禁止输出路径、locator、credential、provider、peer、remote label、revision 或 secret。
- import 默认 dry-run，只有显式 `--apply` 才写入。预算固定为文件最多 1 MiB、最多 4096 个 entry、单个原始 `repo_id` 字符串最多 64 bytes、单个规范化 alias 最多 256 UTF-8 bytes、全体 alias 最多 512 KiB；超出任一整体预算属于全文件错误。顶层不是合法 JSON、`format/version` 不匹配或 `aliases` 不是数组也属于全文件错误。
- 在合法顶层容器内，parse/validate 必须覆盖全部 entry。entry 缺字段、字段类型错误、非法 UUID、unknown local RepoId、invalid alias、重复 RepoId 或单项 admission failure 必须 warning + skip，并在最终 typed summary 中逐项列出 index、可解析时的 RepoId 与原因；不得因一个坏 entry 丢弃其它有效 entry。重复 RepoId 的**全部 occurrence**都必须跳过，不能保留 first/last winner。
- alias 以 trim 后的规范值落盘。dry-run 只给出当时的 projected summary；`--apply` 必须在 alias mutation runtime 的单一 exclusive lock 内重新读取 local membership 与 alias store，和并发 alias set 线性排序，并重新计算完整 summary。
- 所有通过 apply-time 校验的 entry 作为一个原子 accepted batch 更新 alias store；changed row 的 revision 在该批次中基于最新 store 单调递增，新 binding 从 `1` 开始，同值 row 是幂等 accepted/no-change。store-wide commit failure 是全局错误，不得伪装成 per-entry skip 或 partial success。
- import 不得创建 repo、改变 RepoId、locator、workspace、sync、Remote Import 或 credential state。

### 2.2 Branch Identity

- `Local Branch` = `ledger/local/*.redb` 中的本地 repo 事实集合。
- `Remote Branch` = `ledger/remotes/<PeerId>/*.redb` 中的远端镜像事实集合。
- Branch 是 writer identity 的作用域，不是任意命名的 git-style feature branch。

### 2.2.1 Remote Branch Readonly Contract {#remote-branch-readonly-contract}

- Remote Branch 是本机保存的 peer force-mirror / shadow 输入，不是可写工作分支。
- Remote Branch 对所有用户操作、Editor、Source Control、Merge、plugin-host writer 与 Web UI action **MUST** 保持纯只读语义。
- 唯一允许改变 Remote Branch 存储内容的路径是经认证同步协议 ingest peer facts / snapshot；该路径只维护 mirror authority，不是用户写入、不是 merge target，也不得由 Source Control writer 复用。
- 每个 remote shadow 只能持有其目录所绑定物理 source peer 的事实；entry 的 `origin_peer_id` 必须等于该 source，`peer_seq` 必须形成从 1 到 shadow waterline 的连续序列。
- 任何会 append ledger facts、写 pending/staging/commit、确认 pending overlay、应用 merge result 或修改 projection 的操作，若当前 branch 是 Remote Branch，必须 fail-closed 或在 UI 中禁用。
- 需要从 Remote Branch 合并内容时，Remote Branch 只能作为只读 source；用户必须处于 matching Local Branch，并通过 Local Branch writer gate 后选择 peer source。

### 2.3 Tree Identity

- `NodeId` 是文件树节点的权威主键。
- `DocId` 是文件内容实体的权威主键。
- `Path`、`path_cache`、`TreeDelta`、`NodeMeta` 只能是 projection 或 projection cache。

### 2.4 Repo Health

每个 repo instance **MUST** 显式落入以下健康状态之一：

- `Healthy`
- `DegradedProjection`
- `DegradedLocator`
- `DegradedCatalog`
- `Repairing`
- `Quarantined`

其中：

- `RepoHealth` 只描述可由 repo/catalog/locator/projection/repair 证据判断的持久或可重建健康状态；它不承载当前进程 watcher thread 是否仍在运行。
- `Healthy` 是正常 mounted write path 的必要条件，但不是充分条件；还必须同时满足 `RepoMountState::Mounted`。
- `Degraded*` 允许受控只读或 fallback 行为，但必须显式暴露给 runtime。
- `Quarantined` 表示该 repo 不再参与正常 scope 恢复、自动切换和默认列表绑定。
- `RepoMountState` 是 `03_storage/watcher#watcher-contract` 定义的 process-local readiness。watcher `Failed` 不得写入 repo-local Projection Fault store、不得转换为 `DegradedProjection`，也不得改变 Ledger-derived health facts。
- 任一 active repo-local `PROJECTION_FAULTS` row，或仍需幂等收敛的 Remote Import
  Applied/Pending receipt，都使该 repo 进入 `DegradedProjection`；缺 fault 但存在
  Pending receipt 时不得误报 `Healthy`。
- repair/rebuild 必须先进入 `Repairing`，按 exact `RepoId`/locator/workspace marker
  重新验证并完成 materialization，再清除对应 active fault，最后才能回到 `Healthy`。
  单 repo 的 fault、Pending recovery 或 repair 失败不得改变其它 repo 的 health。

在线可写条件固定为：

```text
RepoHealth::Healthy && RepoMountState::Mounted
```

### 2.5 Selector Inputs and Logical Identity {#repo-selector-resolution-contract}

- repo 的逻辑身份基于 `RepoId`；`URL` 或其他 characteristic parameter 仅作为辅助识别线索。
- `RepoName` 相同但 `URL/RepoId` 不同的实例 **MUST** 视为完全不同的 repo。
- 后端接口 **MAY** 接受：
  - `RepoId`
  - `RepoName`
  - `URL`
  - `CurrentScopeFallback`
- 但进入任何底层 repo/document/source-control 算子前，必须解析成唯一 `RepoId`。
- selector 解析必须 UUID-first；`RepoName` 与 `URL` 只能辅助定位，不得覆盖已解析的 `RepoId`。
- branch switch 已持有当前 `RepoId` 时，target branch 必须含有 exact 同 `RepoId`；当前
  host-local alias、URL 或 target 上“唯一 repo”都不得替代。exact RepoId 缺失必须 fail-closed。
- selector 解析出现缺失、重复、metadata drift、URL 歧义时 **MUST** fail-closed。
- repo / branch URL 的 WebDAV、S3 与 S3-compatible 备份展开由 `06_backup.md` 定义；该展开不得改变本章的 UUID-first repo scope 规则。

逻辑 repo 归类规则：

- `RepoId` 匹配
  - 视为同一逻辑 repo 的本地 / remote / shadow 实例。
  - runtime 可以显示 shadow branches、remote mirrors、same-logical-repo peers。
- `RepoId` 不匹配
  - 即使 `RepoName` 或 `URL / characteristic parameter` 相同，也必须视为不同 logical repo。
  - 应进入 multi-root workspace、显式 import 或受控 recovery candidate，而不是混入同一 repo 的 branch/scope。
- `RepoId` 缺失但 `URL / characteristic parameter` 匹配
  - 只能作为发现 / 恢复候选，不能直接 admission 为同一 repo。
  - 必须经过用户确认、ledger header 校验和 `RepoId` 绑定后，才允许进入 writable 或 merge-capable scope。
- `Peer-only Repo`
  - 若只存在于远端且不匹配当前本地逻辑 repo，必须强制只读。
  - 仅允许 copy / inspect / diff / explicit import，禁止直接写入或错误绑定为 local writable repo。

## 3. Storage Layout

### 3.1 Physical Layout

- `ledger/local/<repo_id>.redb`
- `ledger/remotes/<peer_name>/<repo_id>.redb`
- `ledger/.host/projection-locators.toml`
- `<projection_base>/<workspace_segment>/.notegit/`

`projection_base` 与计算出的 workspace root 由 `03_storage/projection.md#projection-locator-contract` 定义；本章只规定 repo identity 与 locator 绑定边界。

`workspace_segment` 在 locator 创建时确定并保持独立于当前 alias；本机 create 可以使用 `<safe_initial_alias>--<repo_id>`，无本地 alias 的首次绑定使用 `<repo_id>`。

### 3.2 Collision Rule

- 同一 branch 下，alias 相同但 `RepoId/URL` 不同的实例 **MAY** 共存；它们的物理 repo DB 与 workspace root 必须按 exact RepoId / locator identity 区分。
- alias selector 命中多个 `RepoId` 时必须 fail-closed，并要求用户选择明确 `RepoId`。
- 物理文件名或 workspace segment 冲突的处理不得改变逻辑 repo identity；如果同一个 `<repo_id>` 同时指向多个不一致文件，必须进入 repair / quarantine。

### 3.3 Catalog Rule {#repo-catalog-contract}

- local repo catalog 与 remote repo catalog 是 selector / listing / switcher 的输入层，不是业务真值层。
- catalog 损坏时 **MUST** 进入 repair 或 fail-closed，不得静默把错误 repo 绑定到当前 scope。
- catalog entry 必须是可读 repo DB 文件，且文件名、repo header、repo metadata 中的 `RepoId` 必须一致。
- catalog/`RepoInfo.name` 不得保存人类 creation label；在当前 schema 中它固定等于 lowercase canonical `RepoId`，仅作为 legacy-shaped machine identity field。当前 alias 只能从 host-local alias runtime 读取，后续 schema 可以在不影响 repo identity 的情况下删除该冗余字段。
- remote catalog 文件名冲突只能通过安全重命名或受控 repair 处理，不得合并不同 logical identity。
- local repo catalog 不得承载 projection base 或 workspace root；Projection Locator 以 `projection_base + workspace_segment` 解析 workspace root。
- Redb v4 的 exact execution stem 固定为 canonical `<repo_id>`。repair 不得从 `RepoInfo.name`、alias 或路径猜测 rename、改写 RepoId、重命名物理 DB 或 workspace。
- workspace admission 必须读取 `.notegit` identity marker 并验证其 `repo_id == resolved RepoId`；路径名匹配但 marker 缺失或不一致时不得进入 mounted write path。

### 3.4 Tree State Storage Model

- tree projection 的推荐内存结构为 flat map：

```rust
struct NodeInfo {
    node_id: NodeId,
    kind: NodeKind,
    name: String,
    parent_id: Option<NodeId>,
    children_ids: Vec<NodeId>,
    path_cache: String,
    doc_id: Option<DocId>,
}
```

- 规则：
  - `NodeId` 是主键
  - `path_cache` 是可重建缓存
  - `children_ids` 与 `parent_id` 共同定义树
  - 文件节点才允许持有 `doc_id`

## 4. Runtime State Machines

### 4.1 Repo Lifecycle

```text
Unresolved
  -> ResolvingSelector
  -> OpeningInstance
  -> ProjectionLocated
  -> Healthy
  -> DegradedLocator
  -> DegradedProjection
  -> Repairing
  -> Healthy | Quarantined
```

约束：

- `ResolvingSelector` 必须先完成 selector -> `RepoId` 解析。
- `OpeningInstance` 必须验证 runtime tables、catalog、projection 依赖。
- `ProjectionLocated` 必须完成 `RepoId -> (projection_base, workspace_segment) -> <projection_base>/<workspace_segment>/` 解析、canonicalize、`.notegit` identity 校验与冲突检查。
- `DegradedLocator` 禁止 watcher、scan、stage、commit、projection writeback。
- `Repairing` 期间禁止把该 repo 作为默认可写 scope 暴露给 UI。

与上述 `RepoHealth` 正交的 process-local mount lifecycle 为：

```text
Unmounted
  -> Transitioning(generation)
  -> Mounted(generation) | Failed(generation, failure)
  -> Transitioning(next_generation)
  -> Mounted(next_generation) | Failed(next_generation, failure) | Unmounted
```

- `Transitioning` 关闭新 workspace-dependent mutation，但不修改 durable repo health。
- `Mounted` 只能由 watcher capture-first startup 的 clean scan pass 发布。
- `Failed` repo 仍可出现在 repo list 中并提供受控只读、export 与 diagnostic；不得被默认 scope 或 writer registration 选择为可写 repo。

### 4.2 Scope Binding

```text
BootstrapUnbound(scope_nonce = 0)
  -> SwitchingRepo(switch_nonce > 0)
NoScope(scope_nonce)
  -> SwitchingRepo(switch_nonce)
RepoBound(repo_id, branch, scope_nonce, catalog_membership_token)
  -> DocBound(doc_id)
  -> SwitchingRepo(switch_nonce) | SwitchingBranch(switch_nonce)
SwitchingRepo | SwitchingBranch
  -> RepoBound(new_repo_id, new_branch, scope_nonce = switch_nonce)
  -> NoScope(scope_nonce = switch_nonce)
```

约束：

- repo switch 与 branch switch 只允许在解析成功后提交到 session。branch switch 的已知
  `RepoId` 只允许 exact target match，不使用 alias/URL/single-repo fallback。
- 新连接尚未提交任何 scope epoch 时唯一允许 `BootstrapUnbound(scope_nonce = 0)`；
  该哨兵态没有 repo identity、writer readiness 或 workspace authority，只保留 login、
  diagnostic 与首个 Create。首个 Create 必须提交严格大于零的 switch nonce，不能把零值
  当作已确认 `NoScope` 或 `RepoBound`。
- `NoScope` 也是带 `scope_nonce` 的已确认 scope epoch；进入它必须提升 nonce，使旧 RepoBound 的延迟消息失效，不能用 `None` 或零值绕开 scope gate。
- 每个 process-local `RepoBound` 必须保存当次 bind 的 `CatalogMembershipToken`。writer admission、new bind 与 scope publication 应用都必须 exact-compare 当前 per-repo membership generation；token 一旦被 durable membership cut 撤销，旧 binding 即使尚未收到 UI 消息也必须立即拒写。
- 旧 scope 的延迟消息不得继续驱动新 scope。
- `last_local_repo` 只允许作为恢复线索，解析失败时必须 fail-closed。
- local writable scope 只有在目标 repo 同时 `Healthy + Mounted` 时才可发布 write-ready。显式进入健康但非 Mounted repo 时只能绑定为 readonly；create 的 mount failure 不自动切换 session。host-local alias set 不改变 session identity/readiness，remove committed partial 按 §7.6 退休旧 scope并保持 fail-closed。

### 4.3 Health Recovery

```text
Healthy -> DegradedProjection -> Repairing -> Healthy
Healthy -> DegradedLocator   -> Repairing -> Healthy
Healthy -> DegradedCatalog    -> Repairing -> Healthy
Degraded* -> Quarantined
```

约束：

- `Repairing` 成功前，不得把 projection fallback 伪装成正常健康状态。
- `Quarantined` repo 不得被 stale scope 自动恢复逻辑再次选中。

### 4.4 Spectator / Readonly Branch State

```text
LocalWritable
RemoteReadonly
ReadonlyDegraded
```

规则：

- remote branch 默认进入 `RemoteReadonly`
- `RemoteReadonly` 允许 read/copy/diff/merge-into-local
- `RemoteReadonly` 禁止 rename/delete/edit/stage/commit into remote mirror
- 若 remote repo 自身 projection 损坏，则进入 `ReadonlyDegraded`，仍不得提升为可写

## 5. Tree Projection Contract {#tree-projection-contract}

### 5.1 Authority Rule

- 树的权威来源是 Structure Facts，不是 path 表。
- `CreateDoc / RenameDoc / MoveDoc / DeleteDoc` 的最终业务事实必须先入 ledger，再由 projection 导出 tree。
- 本地结构批量写入中，ledger append 与 projection apply 必须处于同一个写事务；任一结构事实校验或 projection 失败时，不得留下前序 op 或 path/node projection 残留。

### 5.2 Projection Rule

- `TreeManager` 是内存 projection，不是 authority。
- `TreeDelta` 是 projection diff，不是业务写路径。
- `path_cache` 只允许由 projection builder / repair 流程写入。
- Structure Facts 导出的任意 repo child path 必须通过 `03_storage/projection.md#projection-contract` 的 canonical relative-path 与 existing-ancestor containment gate；坏 path、外指 symlink / junction 或不可 stat / canonicalize 的 ancestor 必须使 tree projection fail-closed，不得写到 workspace root 外部。

### 5.3 Fallback Rule

- docs-only fallback 只能作为受控降级手段。
- fallback 生效时 repo health **MUST** 标记为 `DegradedProjection`。
- fallback 不得被长期视为正常最终状态。

### 5.4 TreeDelta Contract

- `TreeDelta` 只能表达 projection 变化，不是 authority mutation。
- 支持的 delta 类别：
  - `add_node`
  - `remove_node`
  - `rename_node`
  - `move_node`
- delta 构造必须来源于 Structure Facts 应用结果，而不是 handler 直接篡改 path 表。
- 在线 Docs create/copy/rename/delete 与 repo authority mutation 必须进入
  `03_storage/authority#repo-mutation-publication-gate`。批量结构变化完成后只发布一个由
  后端指定刷新范围的 projection recovery；不得由 Web 根据路径或操作类型猜测应刷新哪些投影。
- 多事务旧流程若中途失败且已有事实提交，必须报告 committed-partial、标记必要 degraded
  状态并发布 recovery；不得以普通失败隐藏已经成立的前缀 authority effect。

### 5.5 Sorting Contract

- 树视图构建时 **MUST** 遵循：
  - Folder First
  - Alphabetical
  - Case-Insensitive

## 6. Commands / Inputs / Outputs

### 6.1 Input Types

- `RepoSelector`
  - `RepoId`
  - `RepoName`
  - `RemoteRepoSelector`
  - `CurrentScopeFallback`
- `BranchSelector`
  - `Local`
  - `Remote(PeerId)`

### 6.2 Core Commands

- `SwitchRepo`
- `SwitchBranch`
- `ListRepos`
- `CreateRepo`
- `SetHostRepoAlias`
- `ExportHostRepoAliases`
- `ImportHostRepoAliases`
- `PrepareLocalRepoRemoval`
- `ExecuteLocalRepoRemoval`
- `RepairLocalRepoRemoval`
- `ListShadows`
- `ResolveCurrentScope`
- `RepairLocalRepoCatalog`
- `RepairRemoteRepoCatalogs`
- `RepairStructureProjection`

### 6.3 Output Contracts

- 成功切换必须返回新的 `repo_id / branch / scope_nonce`。
- selector 失败必须返回结构化错误，不得静默回退到“某个看起来像默认值的 repo”。
- repair 失败必须把 repo 标记为 degraded 或 quarantined。

### 6.4 Switching Behavior Matrix

- `Local -> Remote`
  - 输入：`PeerId + RepoSelector`
  - 输出：remote repo bound in readonly mode；所有写入、merge apply 与 Source Control mutation 必须禁用或 fail-closed
- `Remote -> Local`
  - **SHOULD** 优先恢复最近稳定本地 repo
  - 恢复失败时 **MUST** 回到 UUID-first 解析
- `Broken Persisted Scope -> Startup`
  - **MUST** 清理 stale last scope
  - **MUST** 重新 bootstrap 健康 repo 列表

## 7. Recovery / Repair Contract {#repo-health-and-repair}

### 7.1 Selector Recovery

- 如果用户提供 `RepoName`，系统 **MAY** 做别名解析。
- 如果解析结果不唯一或不一致，系统 **MUST** fail-closed。
- 从 `Remote -> Local` 返回时，系统 **SHOULD** 优先恢复最近一次稳定本地 repo。
- “稳定本地 repo” 在在线 writable recovery 中固定表示 `RepoHealth::Healthy + RepoMountState::Mounted`；仅 Healthy 但非 Mounted 的 repo 只能作为显式 readonly/diagnostic target。
- 最近本地 repo 不可解析时，必须回到严格 UUID-first 路径，而不是绑定任意本地 repo。

### 7.2 Catalog Repair {#repo-catalog-repair-contract}

- local / remote repo catalog repair 只能修复 catalog、blank selector、duplicate metadata 与 canonical RepoId machine-field diagnostics；不得从任何历史人类名称恢复 alias。
- repair 不得修改 ledger authority 本身。
- repair 可以补全缺失 URL、分配安全物理文件名；但如果会合并两个 logical repo，必须 fail-closed。host-local alias store 只能由 alias runtime 的显式 set/import/repair command 变更。
- repair 后仍无法形成唯一 `RepoId / URL / filename` 映射时，repo 必须保持 degraded 或 quarantined。

### 7.3 Projection Repair

- structure projection 缺 parent、断链、脏 path cache 时，必须通过 rebuild / repair 处理。
- rebuild / repair 只允许重建 projection tables 与 workspace projection，不得修改 Structure Facts authority。
- 若 Structure Facts authority 本身引用缺失 parent / missing node / cycle / doc identity mismatch，repair **MUST** 输出结构化诊断并 fail-closed；该 repo 必须保持 `DegradedProjection` 或进入 quarantine，直到用户通过导出、重建 repo 或明确的 authority-level 迁移处理。
- repair 失败时 repo **MUST** 退出正常 mounted write path。

### 7.4 Projection Locator Repair

- locator repair 只能创建、替换、删除或校验 host-local Projection Locator。
- locator repair 不得修改 repo ledger facts、repo URL、host-local alias 或 shadow branch identity。
- projection base 变更后，系统 **MUST** 先停止该 repo watcher，再执行 locator 更新、projection materialize / rebuild、watcher restart。
- projection base 变更不需要移动旧 workspace；旧目录只能作为外部数据源，经显式 import / repair 流程进入 pending 或 rebuild。
- host-local alias set/import 不改变 projection base 或 `workspace_segment`，也不触发 workspace realign。`workspace_segment` 在 RepoId 生命周期内永久不变；显式 workspace relocation 只能替换 projection base 并复用同一 segment，同时轮换 locator binding generation 使旧 watcher/admission token 失效。目标已存在、`.notegit` RepoId 不一致或目录冲突时必须 fail-closed。
- locator 缺失或冲突必须保持 `DegradedLocator`，直到用户显式提供可用 base 且计算出的 workspace root 可用。

### 7.4.1 Local Repo Create Contract

- create 必须由 `RepoLifecycleCoordinator` 使用 `Catalog -> Repo(new_repo_id)` lane 编排；handler 只提交 typed intent，不得直接创建 watcher 或修改 supervisor slot。
- create 的唯一 committed/linearization fact 是 `RepoCatalogRuntime` 在短 `Catalog -> Repo(new_id)` lane 内，原子发布 `ledger/.host/repo-catalog/<repo_id>.json` 的 `Normal` membership record 并轮换对应 process-local membership generation。该 per-repo record 是 project-owned bounded JSON v2；conditional cut 只允许读取该单记录当前 revision，再用 same-directory temp + flush + atomic replace + directory sync 发布新状态。此前创建的 canonical `<repo_id>.redb`、locator、workspace marker 与初始 projection 都是不可被正常 listing/admission 观察的 `PreparedRepoCreation`；cut 前失败可以按 prepared manifest 精确清理，不能把 artifact 存在误判为已创建 repo。
- prepared locator 必须通过 `projection_locator_runtime` 的 creation-only typed command 写入：该 command 以
  canonical `<repo_id>.redb` metadata 证明 exact RepoId，只豁免当前 target 的 normal catalog membership，
  不豁免整张 locator map 的路径、冲突或 identity 校验。正常 locator set/query/list 不得接受未入 catalog 的
  target，也不得提供 boolean `allow unknown` 之类的通用逃生口。
- DB identity、locator binding generation 与 workspace marker identity 必须在 permits 外形成 project-owned `RevalidatedRepoCreation`；membership cut 再 exact-compare 该 token、`PreparedRepoCreation`、RepoId 与 typed `Catalog -> Repo` permit，并产出 immutable `RepoCreationCommittedCutPlan`。cut 内除上述单个 bounded membership record 的 exact read / atomic publish 外，不得执行其它 filesystem I/O、scan、join、目录遍历或发送消息。cut 后 alias settlement 与 watcher mount 位于 permits 外；即使 settlement 失败也不得删除 repo 或撤销 membership。
- durable create 已提交后，无论 mount 成功或失败，都只能在 mount 最终 outcome 确定后发布一次 repo-list update；该 update 必须携带最终 `Mounted` 或 readonly/unavailable 状态，不得先广播可写 success 再补发 watcher failure。
- mount 成功时才允许发布可写 scope，并允许当前 session 自动切换到新 repo。
- durable create 已提交但 mount 失败时不得删除新 repo、回滚 Ledger/catalog 或猜测清理 workspace。repo 保留在列表中且只读可见，当前 session 不自动切换，并返回 typed “创建已提交但 workspace ingestion 不可用” partial outcome。

### 7.5 Host-local Repo Alias Contract

- 普通产品 “Rename repo” 只表示 `SetHostRepoAlias`，唯一 target 是当前 host 的 alias store；它不是 repo authority mutation。
- intent 必须携带 exact `repo_id`、`alias` 与 `expected_alias_revision`；stale revision 必须 typed reject，不得 last-write-wins。
- alias runtime 必须先 exact-validate local catalog membership，再执行短 CAS。它不得要求 `Mounted`、不得进入 watcher E2、不得检查 workspace dirty/staged/pending/Projection Fault，也不得写 Ledger/locator/catalog/Remote Import。
- 成功后只发布 backend-produced repo-list/display projection；所有已绑定同一 RepoId 的 session identity、branch、scope nonce 与 writer gate 保持不变。
- alias store 持久化成功而某个连接 publication 失败时，重连后的 list 必须从 runtime 读取新 alias；不得反向回滚 alias。
- alias import/export 的 schema、warning/skip 与 accepted-batch atomicity 唯一归 §2.1.1；Web 只发送 typed set intent，不解析 JSON 或自行决定 skip 原因。

### 7.6 Local Repo Removal Contract {#local-repo-removal-contract}

- Web 与 CLI 的“移除仓库”必须调用同一 ownership-aware `RemoveLocalRepo` service：删除该 host 上属于 Deve 且仅属于目标 local repo membership 的 authority/runtime 对象，同时保留 Projection Workspace 容器及其中除 exact `.notegit/` 外的全部用户或外部对象。它不是 secure erase，也不是删除 workspace 的 `DestroyLocalRepo`。
- `RemoveLocalRepo` 必须解析到唯一 `RepoId`，只能作用于 Local Branch，remote/spectator scope 必须 fail-closed。最后一个 repo 可以移除；不存在其它可用 repo 时，受影响 session 正常进入 `NoScope`，host 继续提供 login、diagnostic 与 Create。
- 产品流程固定为 `PrepareLocalRepoRemoval -> ExecuteLocalRepoRemoval`；不存在 direct Remove lifecycle intent。Prepare 构造并持久化 exact tagged ownership manifest，但不改变 repo membership、Ledger、workspace 或其它业务 authority；它返回 backend-generated preserved/deleted categories 与 typed warnings/blockers，不暴露路径、digest、file identity 或 raw detail。
- 无 blocker 时 Prepare 签发随机 256-bit、五分钟有效、一次性 confirmation token；服务端只持久化 token hash。所有 token 必须绑定 exact preparation id、`RepoId`、catalog membership revision、authority generation、locator/marker identity、manifest digest、scope epoch与optional fallback binding。Web token还绑定authenticated principal/session、connection epoch与server runtime incarnation，只能驻留内存，不得进入URL、browser storage或telemetry；loopback CLI token绑定`LocalCliProxy` principal与server incarnation；offline两次CLI invocation的token绑定canonical authority-root identity、persistent authority-lock file identity、membership/generation与preparation record，不绑定短命CLI process。
- Prepare 与 Execute 使用不同 request id；Execute 必须显式引用 exact preparation id。相同 Prepare request retry重新签发token并立即废止旧token。Execute必须在同一durable preparation/job record内一次性CAS为`ExecuteAdmitted { execute_request_id, job_id, consumed_token_hash }`并fsync后才启动worker；相同Execute request retry返回已存job/result，不重复删除。token缺失/错误、过期、已消费或issuer/principal/connection不匹配返回typed invalid/expired，generation/scope/catalog/manifest/marker drift返回typed stale。startup recovery必须恢复已admitted但未完成的job。旧request id在receipt裁剪后没有当前membership-bound token，不能作用于后来重新admission的同RepoId；不得为此保留永久replay fence。
- 删除集合是封闭的project-owned tagged manifest：canonical local DB与workspace `.notegit/`记录已解析绝对路径、父目录identity、canonical containment、原始FileId/inode与随机manifest-bound quarantine id；Remote Import capture只记录owner-issued artifact token，逐项identity inventory由该owner写入capture root内的single durable plan slot并以digest绑定，不能膨胀通用lifecycle receipt；Projection Locator、host-local alias与catalog tombstone只记录各owner签发的conditional row token。每个owner root从quarantine id派生唯一同父reserved name，Prepare证明该name不存在，Execute只允许no-replace move；不得cross-volume copy、覆盖已有entry或扫描选择目标。`.notegit` marker是唯一例外：它从已固定identity的`.notegit` source parent移动到已固定identity的workspace destination parent，仍须same-filesystem、no-replace并复核两端parent与moved marker identity。watcher/provider/session slot是typed retirement action而不是filesystem target；Projection Fault与Remote Import session row随canonical Redb整体退休，不另列path。未来owner必须先登记typed removal plan，其artifact才可加入manifest；不存在`repo runtime artifacts` catch-all。DB必须是regular、non-symlink/non-reparse file并绑定原membership/genesis identity；顶层`.notegit/`必须是identity-unchanged real directory且不是symlink/junction/reparse point。不得从alias、workspace目录名、glob、目录扫描、文本RepoId marker或stale path cache推导目标。
- 明确保留集合包括：workspace root、Markdown、附件及其它非`.notegit` workspace child、`.git/`、`.gitignore`、`.deveignore`、remote shadow branch、全局peer identity/config、persistent authority lock pathname与其它RepoId的任何对象。operator提供的`backups/projection-workspace`外部恢复输入只在reserved removal roots之外获得保留保证；active recovery input若与exact `.notegit`、canonical Redb、其reserved quarantine或Remote Import target重叠，Prepare必须block。当前首发没有managed local-backup runtime，remove不得临时新建或删除一个。实现不得递归删除workspace root；`.notegit` identity marker必须先通过上一条的受控跨目录rename移动到workspace sibling quarantine，tree随后同父整体quarantine并通过hardened no-follow recursive removal清理，marker quarantine最后删除。child link/reparse entry只能作为tree内entry删除，绝不进入target。quarantine是内部destructive intermediate，不是backup、recycle bin、restore或normal repo discovery输入。
- remove是明确丢弃本机Ledger history与Deve workflow state的不可撤销destructive intent；首发没有受支持的Ledger history restore。确认文案必须明确“Ledger/runtime会删除，workspace文件与`.git`会保留”，不得把export或operator recovery input描述为可验证restore。
- blocker至少包括：active Projection Fault或pending/failed writeback；authority repair、writer、Source Control、External Apply或Remote Import Apply持有mutation/authority lease；RepoId/catalog/locator/marker/path关系不唯一；`.notegit` ownership不可证明、顶层identity replacement或unsafe reparse；recovery input与removal target重叠；Applied Remote Import outcome为Pending/Degraded；无法取得DB quiesce/exclusive retirement proof。
- Remote Import artifact的唯一owner仍是`remote_import_runtime`。Prepare生成owner-issued observation；Execute关闭新产品写门并quiesce provider、等待既有capture封存或安全中止后，才可exact revalidate并封存immutable cleanup plan：`Preparing/Ready/Stale/Failed`是warning且允许owner cleanup capture；`Applied/Written`或`Discarded`允许artifact cleanup；`Applied/Pending` blocker；`Applied/Degraded`由Projection Fault blocker；corrupt/unknown为`RepairRequired`。Removed cut后owner只能把exact repo artifact root整体移入同父quarantine，再按sealed plan做artifact-only cleanup；不得逐项pathname unlink，也不得修改即将随canonical Redb删除的session row。lifecycle coordinator不得读取/删除artifact path或重定义session state。
- pre-cut顺序固定为：durable `ExecuteAdmitted` -> reserve `Transitioning(generation)`并关闭新产品写门 -> exact revalidate catalog/locator/alias/`.notegit`/DB -> quiesce provider task并等待capture稳定 -> seal + fsync Remote Import owner plan -> watcher E2 final-state reconcile -> final exact owner-plan revalidation -> authority `Quiescing`并在30秒内drain ordinary leases -> owner-internal exclusive retirement proof。该阶段失败必须按逆序补偿：恢复同generation Active、补偿性重启exact旧watcher、逻辑失效sealed owner plan、恢复exact provider generation，最后才释放Transitioning reservation与产品写门。plan invalidation失败时不得恢复provider或释放写门；invalidation只改变single owner-plan slot的logical epoch，不执行pathname delete。任一补偿失败都返回typed readonly/repair partial并保持写门关闭，不得声称普通NotCommitted+Active；不得隐藏repo、清理locator或进入membership cut。
- durable membership authority cut必须是repo-scoped conditional apply：exact manifest先以same-directory temp + flush + atomic replace + directory sync发布为`ManifestPrepared`；Execute admission随后按上一条执行。最终在短`Catalog -> Repo(target)` lane内exact-compare preparation/token/permit/retirement proof，把record从`Normal`切为引用`request_id + manifest_digest`的transient `Removed` tombstone并轮换membership generation。该cut内不得执行其它filesystem I/O、session遍历、network、scan、watcher I/O、await或长时计算。
- `Removed` tombstone是cleanup recovery线性化证据，不是长期soft-delete。完整持久顺序固定为`ManifestPrepared + fsync -> ExecuteAdmitted + fsync -> Transitioning/provider quiesce/owner seal/watcher E2 -> authority Quiescing + drain -> CutAttempted + fsync -> Removed(request_id, manifest_digest) + fsync -> CutObserved + fsync -> owner-specific quarantine checkpoint/cleanup receipt + fsync -> CleanupComplete + fsync -> exact tombstone retirement -> TerminalCandidate(publication disabled) + fsync -> Retired/release authority lock handle -> terminal receipt/publication enabled + fsync -> best-effort session/network publication delivery`。tombstone retirement必须exact-compare RepoId/state/revision/request_id/manifest_digest；persistent lock pathname永不删除。publication failure只能形成control-plane delivery debt，不能无限持有lock或阻止same-RepoId readmission。
- cleanup顺序固定为`Remote Import root quarantine/delete -> retire process-only watcher/provider/session slots -> .notegit marker quarantine/tree quarantine/tree delete/marker delete -> canonical Redb quarantine/delete -> conditional locator row -> conditional alias row -> CleanupComplete -> exact catalog tombstone -> TerminalCandidate -> authority Retired -> durable terminal job result`。每个filesystem owner API一次只推进一个durable cut，返回opaque checkpoint；coordinator只持久化typed checkpoint，不拥有store path解析或直接写权限。terminal result按既有bounded receipt policy保留以支持lost-response replay，不得作为本job cleanup target。
- settlement是可恢复saga。任一步失败必须保留`primary + cleanup[]`、逐owner checkpoint与`deleted/already_absent/retired/failed` receipt，返回`CommittedPartial`或`RepairRequired`；cut后永不remount、重建alias/locator或回滚membership。`CutAttempted`在得到exact catalog truth前永远是active debt，不得由generic worker terminalize或裁剪；exact Removed truth必须收敛为`CutObserved`；exact Normal truth表示catalog cut未提交，startup必须完成可证明的逆序补偿并终结为`NotCommitted`，补偿不能唯一证明时进入`RepairRequired`。每个owner的恢复分类必须同时读取durable checkpoint：`Prepared`只接受original exact + quarantine missing；`Quarantined`接受original missing + quarantine exact；若已持久化`Quarantined`且唯一下一动作是delete，则original missing + quarantine missing可重建`Deleted`；both-present、没有前置checkpoint证明的both-missing、changed identity、unsafe reparse或越界进入`RepairRequired`。`.notegit`还必须依次持久化`Prepared -> MarkerQuarantined -> TreeQuarantined -> TreeDeleted -> MarkerDeleted`，每一状态分别复核marker original/quarantine与tree original/quarantine的唯一合法组合。remaining target只有在原owner-issued durable identity未变且仍位于原containment时，才可由removal-repair dry-run签发短期token并经显式`--apply`重新授权。repair观察、token hash、五分钟expiry、观察digest与issuer binding必须写入同一host-local removal preparation record v4；proxy issuer绑定operator principal与当前server lifecycle incarnation，offline issuer绑定canonical authority root与当前独占的persistent lifecycle receipt lock identity。重新观察或任何owner checkpoint变化都立即废止旧token；Apply在同一lifecycle worker内一次性消费token并重新执行原sealed plan，同时持久化consumed token hash/issuer，使同一token在响应丢失、worker运行中或终态后只能重放同一job/result而不能再次cleanup。显式offline repair必须先取得lifecycle receipt lock，再初始化RepoManager、scan或watcher，并从自动startup recovery中排除该request，避免repair preview与同一debt自我竞态；只有锁已由live server持有时才转authenticated proxy，`.host/main_port`只用于验证该live owner endpoint，不能单独充当owner存活证据。它不是owner-plan rebinding：顶层`.notegit`、canonical DB、Remote Import root或parent identity replacement永远不可apply，即使文本marker/RepoId相同；DB还必须匹配原membership/genesis identity。旧v1-v3开发期removal record直接fail closed，不提供adapter。
- durable terminal result采用two-phase finalization：先持久化`TerminalCandidate`且保持publication disabled/committed debt，再由authority owner把slot转为`Retired`并释放OS lock，最后原子标记terminal receipt与publication enabled。retirement或最终receipt失败不得向产品发布success；重启必须重新取得persistent lock并从DB/catalog/quarantine truth恢复finalization。
- Prepare可以接收用户明确选择的optional fallback RepoId；backend只有在其`Healthy + Mounted` membership可证明时才返回opaque exact fallback binding，并把它绑定进confirmation token。Execute只能回显该binding与switch nonce，不能提交新fallback或仅按RepoId重查。cleanup完成后binding仍exact时发起者可以切换；这是optional convenience，不是removal前置条件。没有binding或apply-time binding已stale时，所有仍绑定removed RepoId的session各自提升epoch并进入`NoScope`；成功删除不得因fallback失效降级成错误、由backend自动选取或选择第三个repo。
- durable remove成功后目标repo不得重新启动watcher，不参与normal list/default recovery。later same-RepoId admission必须遵守authority contract的two-stage owner-prepared reincarnation：完整terminal settlement产生的live `Retired(prior_generation, expected_lock_identity)`本身是same-process admission proof，不依赖可裁剪的旧job receipt；先single-flight进入`Reopening`，再以existing-only persistent lock、expected identity与canonical DB absence proof准备新DB并进入`ReopeningPrepared`。authority、locator、marker各自保有owner capability，composition layer构造同时绑定DB physical/genesis、lock、locator store+row revision及workspace root+marker identity的`PreparedRepoIdentity`，existing prepared-create/catalog cut只发布该fresh Normal membership。lifecycle Transitioning permit必须覆盖prepare至activation；独立短activation guard按`locator read capability -> catalog -> authority slot`固定顺序冻结全部project-owned identity owner，并在bounded no-follow revalidation、exact durable record、fresh membership、digest与reservation全部一致时CAS为冻结的next generation；不得混入repair、scan或unbounded filesystem work。catalog cut响应未知必须按exact durable truth分类，只有证明Normal absent才能owner-specific rollback；exact Normal继续activation/cold-start recovery，其它truth是lock-held repair debt。完全移除后若进程重启导致`Retired` proof消失，本轮不得仅凭lock pathname或已裁剪receipt重建same RepoId。任何旧lease、token、request或cleanup capability均不可复用。保留的workspace只是用户文件集合；重新纳入必须走显式create/import与正常reconciliation/Remote Import，不得从残留path或旧request猜测原membership。
- 本轮唯一compiled producer是server composition内的typed `RepoLifecycleCoordinator::readmit_retired_repo`，它只能在调用方已拥有exact RepoId、live Retired proof与Transitioning permit时调用core owner API；integration producer走同一非test实现。F4/v5 `RepoLifecycleIntent::Create`继续只生成fresh RepoId，Remote Import也不伪装成readmission；本轮不新增WS/CLI/UI入口。未来若产品要让用户显式跨重启导入既有RepoId，必须另行批准typed transport与durable lineage proof，前端仍只能提交RepoId intent，不能持有authority/identity/activation能力。
- session runtime只能消费immutable `RepoRemovalSettledPublication`：在Catalog permit外撤销旧binding、按connection独立生成`NoScope` epoch并发布最终RepoList/outcome。不得清除editor pending overlay，不得在lifecycle coordinator内持有connection map或执行O(N) fan-out。
- remove cleanup出现partial outcome时必须按lifecycle receipt与RepoId重读transient tombstone、DB identity、locator、alias、workspace marker/`.notegit`，并向Remote Import owner查询typed removal state；混合事实进入exact-manifest repair，不得用单一路径存在性猜测durable remove是否成立。workspace root及其保留child不参与“是否已移除”的判断。

### 7.6.1 Remote Import Repo Lifecycle {#remote-import-repo-lifecycle}

- provider task 绑定 `(RepoId, provider_generation, CatalogMembershipToken)`。acquire 必须在 provider slot mutex 内 exact-compare caller 提供的 membership token，并把 token 固化进 task slot；completion 必须再次 exact-compare catalog membership、session identity 与 generation。stale acquire/completion 只能 fail-closed 或 cleanup 自己的临时 capture，不得发布 session、写 Ledger 或改变 mount slot。
- remove 在 authority Quiescing、catalog cut 与 locator cleanup 之前，必须先 quiesce 对应 provider task，再执行 watcher E2 final-state reconcile。quiesce 不能持有 supervisor map mutex、catalog/repo permit、mutation lane 或 publication lane。
- host-local alias 修改不参与 provider generation，也不改变 head/locator/ignore snapshot；它不得把 candidate 转为 `Stale`。
- create/remove 的 partial outcome 必须按 `RepoId` 重读 catalog、metadata、locator、workspace marker 与 Remote Import owner removal state。只有 pre/post truth 唯一一致时才能继续 mount、capture 或 exact cleanup；混合事实进入 repair，禁止猜测删除、回滚或重绑。

### 7.7 Catalog Conflict Repair

- 同名 display repo 但不同 logical identity 时，只允许修复 catalog/name hint drift，不得合并 authority。
- remote repo selector 若只能唯一解析到一个健康 remote repo，可做受控 fallback；一旦出现歧义，必须 fail-closed。

### 7.8 Startup Scan Contract

- startup materialize 遇到坏 repo 时，不得拖垮整个服务。
- 坏 repo 必须显式标记 degraded/quarantined。
- 被跳过的 repo 不得继续参与自动 scope 恢复。
- watcher repo-local start failure 只记录该 repo `RepoMountState::Failed`；其它健康 repo 继续 mount。host-fatal 仅允许 supervisor invariant、generation corruption、thread/resource exhaustion 或 runtime coordination failure 等 typed 分类。
- bootstrap 允许零个 local repo；此时 watcher `expected=0` 是 healthy，新连接保持
  `BootstrapUnbound(scope_nonce=0)` 并保留 login、diagnostic 与 Create，不得伪装成已确认
  `NoScope` 或 writer-ready。存在 local repo 但零个 Mounted 只表示 repo-local ingestion 不可用，
  不自动构成 host-fatal；只有 supervisor invariant、generation corruption、
  thread/resource exhaustion 或 runtime coordination failure 等 typed host-fatal 才终止 server。

### 7.9 Repo Lifecycle Coordinator {#repo-lifecycle-coordinator}

`RepoLifecycleCoordinator` 是 create/remove 与 watcher mount 的唯一业务编排者。host-local alias
不进入该 runtime。transport handler 只能向 host-owned `RepoLifecycleJobRuntime` 提交 typed
intent；job 一经 admission，不得因 WS/HTTP/CLI transport cancellation 丢失 owner。

固定 lifecycle transaction 为：

```text
typed intent -> host-owned lifecycle job
  -> admit + nonblocking reserve Transitioning(generation)
  -> release permits
  -> provider quiesce / owner-plan seal / watcher E2 / bounded filesystem prepare
  -> reacquire Catalog -> Repo
  -> exact revalidation + minimal durable authority cut
  -> immutable committed-cut plan
  -> release permits
  -> filesystem settlement / mount finalization
  -> immutable settled publication
  -> RepoLifecyclePublicationSink / RepoSessionRuntime conditional publication
```

规则：

- `RepoLifecycleJobRuntime` 只拥有 bounded single-flight jobs、completion、shutdown/join 与
  transport-independent convergence；它不拥有 Ledger/Redb facts、watcher backend、connection
  map 或 network fan-out。handler drop 只能停止等待，不能取消已接收 job。
- 每个 create 或 execute-remove intent 必须携带 caller-generated opaque UUID `request_id`。runtime admission 原子分配
  `job_id`；create 同时分配 immutable target `RepoId`。同一 request_id 只能绑定同一 operation
  与规范化参数，参数不一致必须 typed reject；相同 retry 返回既有 job/terminal result，不能重复
  create/remove。active job、cleanup/repair debt 与仍处于 normal catalog 的 create receipt 不得裁剪。普通
  terminal completion 至少保留 24 小时且最多 1024 条；destructive remove 的旧请求安全由 C2′
  membership-bound confirmation token 保证，receipt 裁剪后旧 request_id 不能重新签发或重用旧 token。重连方使用 request_id 查询现存结果。
- lifecycle receipt 是 host-local control-plane state，不进入 Ledger/sync/provider/Remote Import。
  restart 不复活原 worker 或重跑未进入 cut 的中断 job；它先用 receipt 中的 request_id/target RepoId 与
  catalog/locator/marker truth 将结果分类为 NotCommitted、CommittedPartial 或 RepairRequired。只有已进入
  remove cut、manifest 与所有 remaining target fingerprint 完全匹配的 settlement 才可由独立 startup recovery
  自动收敛；可证明仍属同一 RepoId 但发生身份漂移的 target 只能进入 dry-run repair 并签发新 token，未知身份永远
  `RepairRequired`。进程重启后的相同 request_id 不得创建第二个 RepoId 或作用于新 membership incarnation。
- `RepoLifecyclePublicationSink` 是 host-owned narrow consumer；job runtime 在 transport waiter
  之外把 settled publication 交给它。shutdown 必须先停止 admission，再等待所有已接收 job
  达到 terminal/repair outcome 并把可发布结果交给 sink。transport handler 只是可丢弃 observer，
  不能成为 committed plan 的唯一消费者。
- create/remove 必须具有 project-owned `Prepared*Lifecycle`。长时 scan/join/filesystem I/O
  全部位于 permits 外；authority cut 内只允许 exact revalidation 与最小 durable mutation。
- durable cut 必须立即产生 immutable `RepoCreationCommittedCutPlan` 或
  `RepoRemovalCommittedCutPlan`；该对象只描述已经线性化的 authority truth，不伪造未来 mount
  outcome。settlement 完成后再组合为 immutable `RepoCreationSettledPublication` 或
  `RepoRemovalSettledPublication`，其中携带最终 Mounted/readonly、cleanup debt 与 initiator
  outcome。cut 后任意 settlement failure 都是
  `CommittedPartial/RepairRequired`，不得重新返回普通 pre-commit failure或恢复旧 membership。
- 只有能证明未进入 cut 的失败才可以补偿，结果必须保留 `primary + cleanup[]`。cut 内或 cut
  边界发生的 `spawn_blocking` panic/JoinError 一律是 outcome unknown：必须先读取 operation-specific
  完整 truth，只有唯一 pre-cut truth 才能归还 reservation/恢复旧 watcher；post-cut truth 继续
  settlement，mixed truth 进入 repair。transport disconnect 不影响 job owner。进程崩溃不继续
  执行 job，而从 durable receipt 与 RepoId-scoped truth 收敛 completion。
- publication delivery failure 只形成 `publication_pending` control-plane debt，由 sink 有界重试并
  可在重连时从 settled publication 重放；它不得被误标为 repo durable repair。DB/locator/marker/
  watcher settlement failure 才形成 repo-scoped cleanup/repair debt。
- `CatalogMembershipToken` 是 process-local readiness token，由 `RepoCatalogRuntime` 唯一维护，固定绑定 `(runtime_instance_identity, RepoId, per_repo_membership_generation)`。create/remove/recover/repair 等 typed API 只有在改变该 `RepoId` 的正常 catalog membership 时才递增对应 generation；无关 repo 的 catalog mutation 不得使 token 失效。若 catalog repair 无法可靠界定受影响 RepoId 集合，必须提升 runtime instance identity，使本进程全部旧 token fail-closed。`RepoLifecycleCoordinator` 只读取和 exact-compare token。它不得写入 Ledger、Redb schema、Projection Locator、sync facts 或 wire payload，进程重启后旧 token 自动失效。
- `Catalog -> Repo` 锁序与 `03_storage/authority#repo-mutation-publication-gate` 一致；反向嵌套必须 fail-closed。并发 lifecycle intent 命中 `Transitioning` 时返回 typed busy，不得等待时持有 catalog/repo permit。
- watcher/filesystem I/O、scan、join、await、mutation lane 与 publication 期间不得持有 supervisor map mutex。Catalog/Repo permit 也不得跨长时 I/O 或 await。
- lifecycle 使用专用 deferred publication；repo list、scope switch 与 recovery signal 只有在最终 mount outcome 已知后才可 enqueue。
- E2 或 startup/reconcile 产生的 generation-bound `WatcherRefresh` 在 slot 为 `Transitioning` 时必须进入同一 deferred publication 并 coalesce；coordinator 只在 exact revalidation 与最终 mount outcome 完成后决定 enqueue，remove success、stale generation 或进入 repair 时必须 drop。不得在 durable lifecycle outcome 前直接映射为 `FsChangeDetected` broadcast。
- partial outcome 一律以 `RepoId` 为锚重新读取 catalog、repo metadata、Projection Locator 与 workspace `.notegit` marker。只有 pre/post 事实唯一一致时才允许继续；混合事实进入 repair。
- coordinator 不拥有 Ledger、watcher backend 或 UI state；它只编排既有 authority mutation gate、projection/locator runtime 与 `WatcherSupervisor` typed API。

Create/remove 的事实矩阵固定如下；catalog membership record 是唯一 normal listing/admission authority，目录扫描不得替代它：

| Operation phase | Allowed durable/process facts | Recovery rule |
|---|---|---|
| create prepare | job receipt=`Preparing`；unlisted canonical DB/genesis；validated locator；workspace marker/projection；prepared manifest/digests | catalog record absent 必须按 exact prepared manifest 清理或标记 cleanup debt；不得正常 listing/mount |
| create cut | atomic publish per-RepoId catalog record=`Normal`；rotate membership generation；`RepoCreationCommittedCutPlan` | record=`Normal` 即 committed；继续 alias/mount settlement，不能删除 repo |
| create settle | alias best-effort CAS、watcher mount、settled publication/completion | alias 失败回退 display RepoId；mount 失败 readonly；两者都不撤销 catalog membership |
| remove prepare | active catalog record；exact ownership manifest/fingerprint；Remote Import owner removal plan；five-minute one-time token bound to membership/authority/scope/runtime identity | Prepare 不改变 membership；Execute 必须消费仍 valid 且 exact 的 token，否则不进入 quiesce/cut |
| remove cut | `CutAttempted`先fsync；atomic replace same record=`Removed` transient tombstone；rotate membership generation；随后fsync `CutObserved` | exact Removed 即 committed并重建`CutObserved`；exact Normal证明cut未提交时执行可证明的逆序补偿并终结`NotCommitted`；其它truth保持active debt并进入`RepairRequired`；旧token永久fail-closed，禁止重启removed watcher |
| remove settle | authority runtime retire exact DB leases；Remote Import、`.notegit`、Redb owner按manifest-bound同父quarantine checkpoint收敛；conditional删除locator/alias与tombstone；TerminalCandidate后retire authority再开放publication | cleanup/finalization failure保持exact repair debt；保留workspace/.git/remote shadow/operator recovery input；不得恢复normal membership或发布伪success |

所有 recovery 必须至少比较 catalog membership record、canonical DB identity、locator binding、workspace marker、Remote Import active/cleanup state 与 lifecycle receipt。事实不能唯一归入表中的一行时必须 `RepairRequired`，不得凭路径存在性猜测。

## 8. Forbidden Patterns

- 直接用 `RepoName` 或 `Path` 驱动底层业务算子。
- 在 switcher / listing handler 里静默选择“第一个可用 repo”。
- 让 projection fallback 长期替代真正 repair。
- 让 metadata/path table 成为 rename/move/delete 的主写路径。
- 让 UI 直接根据名字推断 repo identity。
- 把 remote readonly repo 误暴露为可写 source。
- 让 alias、URL、全局 vault root 或 `ledger_dir` 推断 projection base、workspace segment 或跨宿主 repo identity。
- 在 peer sync、Remote Projection transport 或 Remote Import manifest 中传输 host-local alias。
- 让 alias set/import 停止 watcher、移动 workspace、写 Ledger 或改变 Remote Import state。

## 9. Runtime Boundary

### 9.1 Authority Layer

职责：

- repo facts
- structure facts
- append validation

### 9.2 Projection / Repair Layer

职责：

- tree projection
- docs fallback
- structure repair
- startup materialize

### 9.3 Scope Runtime Layer {#repo-scope-runtime}

职责：

- scope binding
- selector resolution
- last-local recovery
- stale scope cleanup

### 9.3.1 Lifecycle Coordination

职责：

- `RepoLifecycleJobRuntime` 从 transport 接收 typed create/remove intent并稳定拥有其完成生命周期。
- `RepoLifecycleCoordinator` 编排 prepared lifecycle、Catalog/Repo authority cut、projection/locator runtime 与 `WatcherSupervisor`。
- cleanup settlement 只能调用 authority storage、locator、alias、catalog 与 Remote Import owner 的 typed prepare/revalidate/conditional command；coordinator 不拥有这些 store 的路径解析或删除权限。
- handler、Web shell 与 repo scope runtime 不得直接 start/stop watcher，也不得根据 mount failure 猜测 rollback。
- coordinator 返回 immutable publication plan；`RepoSessionRuntime` 只在最终 mount outcome 已知后 conditional-apply repo list/scope 结果。view layer 只渲染 typed committed/partial/unavailable outcome。

### 9.3.2 Host-local Alias Runtime

职责：

- `host_repo_alias_runtime` 唯一拥有 alias store、CAS、JSON import/export 与 typed warning report。
- repo catalog、projection locator、sync、Remote Import、watcher 与 view 不得复制 alias mutation authority。
- browser/CLI 只发送 typed intent或读取 deterministic export；前端不得校验 alias、解析失败 detail 或推断 skipped entry。

### 9.4 View Layer

职责：

- 仅展示与发出切换意图
- 不得自行推断 repo authority

## 10. Refactor Target

长期 repo 逻辑保留四个 repository core runtime：

- `repo_catalog_runtime`
- `projection_locator_runtime`
- `repo_scope_runtime`
- `projection_repair_runtime`

其中 `projection_locator_runtime` 在 repository 责任带中保持独立命名和唯一 host-local
locator ownership，但在 storage 顶层分层中归属于
`projection_persistence_runtime` 子 runtime；该父子关系不得把 locator 提升为 ledger、
projection 内容或 writeback authority。

host composition 另有两个正交 runtime：`host_repo_alias_runtime` 只拥有本地 display mapping，
`repo_lifecycle_job_runtime` 只拥有 create/remove job convergence；二者不增加 storage authority
层，也不合并进上述四个 core runtime。`RepoManager`、CLI switcher handlers 与 `use_core`
effects 不得共享隐式 repo scope、alias mutation 或 lifecycle ownership。

## 本章相关命令

- `P2P: Switch to Peer`
- `P2P: Establish Branch`

## 本章相关配置

- `repo_creation_projection_base`：可选 absolute path。仅在 host 没有 current local locator 可复用时，
  作为 Create 的默认 projection base；它不是 workspace authority。零 repo server 可以在缺少该配置时
  正常启动，但 Create 必须返回 `REPO_CREATION_PROJECTION_BASE_REQUIRED`。

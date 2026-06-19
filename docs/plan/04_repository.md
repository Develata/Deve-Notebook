# 04_repository.md - 仓库与分支工程蓝图

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-05-30`
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

- `RepoId` 是仓库权威身份，UUID-first。
- `RepoName` 是绑定到 `RepoId` 的可变显示名 / workspace alias，不是身份。
- `URL`、`Selector` 只允许作为输入别名或恢复线索。
- 任何进入业务算子的 repo 输入，在执行前 **MUST** 解析为 `RepoId`。
- 所有 repo instance admission 都必须反向验证 `ledger.repo_id == resolved RepoId`；任何 name/path/url 与 ledger identity 不一致的情况都必须 fail-closed。

`RepoName` 与 `RepoId` 的绑定必须显式建模：

```text
RepoNameBinding = {
  repo_id,
  repo_name,
  name_epoch,
  changed_at_seq,
}
```

约束：

- `RepoNameBinding.repo_id` 必须等于 repo ledger header / genesis metadata 中的 `RepoId`。
- `repo_name -> RepoId` 只能是 catalog index，不是 authority。
- repo rename 必须是显式 authority write：`RenameRepo { repo_id, old_name, new_name, expected_name_epoch }`。
- `expected_name_epoch` 校验失败表示 stale rename intent，必须 reject，不得以路径名或 URL 重试绑定。
- repo rename 只改变 `RepoNameBinding` 与由其派生的 display/workspace segment；不得改变 `RepoId`、branch identity、shadow identity 或 remote attribution。

### 2.2 Branch Identity

- `Local Branch` = `ledger/local/*.redb` 中的本地 repo 事实集合。
- `Remote Branch` = `ledger/remotes/<PeerId>/*.redb` 中的远端镜像事实集合。
- Branch 是 writer identity 的作用域，不是任意命名的 git-style feature branch。

### 2.2.1 Remote Branch Readonly Contract {#remote-branch-readonly-contract}

- Remote Branch 是本机保存的 peer force-mirror / shadow 输入，不是可写工作分支。
- Remote Branch 对所有用户操作、Editor、Source Control、Merge、plugin-host writer 与 Web UI action **MUST** 保持纯只读语义。
- 唯一允许改变 Remote Branch 存储内容的路径是经认证同步协议 ingest peer facts / snapshot；该路径只维护 mirror authority，不是用户写入、不是 merge target，也不得由 Source Control writer 复用。
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

- `Healthy` 才允许正常 mounted write path。
- `Degraded*` 允许受控只读或 fallback 行为，但必须显式暴露给 runtime。
- `Quarantined` 表示该 repo 不再参与正常 scope 恢复、自动切换和默认列表绑定。

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
- `<projection_base>/<safe_repo_name>--<repo_id>/.notegit/`

`projection_base` 与计算出的 workspace root 由 `03_storage/projection.md#projection-locator-contract` 定义；本章只规定 repo identity 与 locator 绑定边界。

其中 `<safe_repo_name>` 只是由当前 `RepoNameBinding.repo_name` 派生的人类可读路径段；完整 `<repo_id>` 必须参与物理路径命名，保证同名 repo 不发生路径碰撞。

### 3.2 Collision Rule

- 同一 branch 下，`RepoName` 相同但 `RepoId/URL` 不同的实例 **MAY** 共存；它们的物理 repo DB 与 workspace root 必须因完整 `RepoId` 后缀而不同。
- `RepoName` selector 命中多个 `RepoId` 时必须 fail-closed，并要求用户选择明确 `RepoId`。
- 物理文件名或 workspace segment 冲突的处理不得改变逻辑 repo identity；如果同一个 `<repo_id>` 同时指向多个不一致文件，必须进入 repair / quarantine。

### 3.3 Catalog Rule {#repo-catalog-contract}

- local repo catalog 与 remote repo catalog 是 selector / listing / switcher 的输入层，不是业务真值层。
- catalog 损坏时 **MUST** 进入 repair 或 fail-closed，不得静默把错误 repo 绑定到当前 scope。
- catalog entry 必须是可读 repo DB 文件，且文件名、repo header、repo metadata 中的 `RepoId` 必须一致。
- catalog 中的 `RepoName` 只缓存当前 `RepoNameBinding`；缓存漂移时只能从 ledger authority 修复，不得反向改写 authority。
- remote catalog 文件名冲突只能通过安全重命名或受控 repair 处理，不得合并不同 logical identity。
- local repo catalog 不得承载 projection base 或 workspace root；projection base 必须通过 host-local Projection Locator 解析，workspace root 必须由 base、当前 safe repo name 与完整 `RepoId` 计算。
- Projection Locator 的 `repo_name_hint` 只能作为诊断信息；不得替代 `RepoId` 绑定。
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
- `ProjectionLocated` 必须完成 `RepoId -> projection_base -> <projection_base>/<safe_repo_name>--<repo_id>/` 解析、canonicalize、`.notegit` identity 校验与冲突检查。
- `DegradedLocator` 禁止 watcher、scan、stage、commit、projection writeback。
- `Repairing` 期间禁止把该 repo 作为默认可写 scope 暴露给 UI。

### 4.2 Scope Binding

```text
NoScope
  -> RepoBound(repo_id, branch, scope_nonce)
  -> DocBound(doc_id)
  -> SwitchingRepo | SwitchingBranch
  -> RepoBound(new_repo_id, new_branch, new_scope_nonce)
```

约束：

- repo switch 与 branch switch 只允许在解析成功后提交到 session。
- 旧 scope 的延迟消息不得继续驱动新 scope。
- `last_local_repo` 只允许作为恢复线索，解析失败时必须 fail-closed。

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
- 最近本地 repo 不可解析时，必须回到严格 UUID-first 路径，而不是绑定任意本地 repo。

### 7.2 Catalog Repair {#repo-catalog-repair-contract}

- local / remote repo catalog repair 只能修复 catalog、name hint drift、blank selector、duplicate metadata。
- repair 不得修改 ledger authority 本身。
- repair 可以补全缺失 URL、把 catalog display hint 纠正为 ledger 中的当前 `RepoNameBinding`、分配安全物理文件名；但如果会合并两个 logical repo，必须 fail-closed。
- repair 后仍无法形成唯一 `RepoId / URL / filename` 映射时，repo 必须保持 degraded 或 quarantined。

### 7.3 Projection Repair

- structure projection 缺 parent、断链、脏 path cache 时，必须通过 rebuild / repair 处理。
- rebuild / repair 只允许重建 projection tables 与 workspace projection，不得修改 Structure Facts authority。
- 若 Structure Facts authority 本身引用缺失 parent / missing node / cycle / doc identity mismatch，repair **MUST** 输出结构化诊断并 fail-closed；该 repo 必须保持 `DegradedProjection` 或进入 quarantine，直到用户通过导出、重建 repo 或明确的 authority-level 迁移处理。
- repair 失败时 repo **MUST** 退出正常 mounted write path。

### 7.4 Projection Locator Repair

- locator repair 只能创建、替换、删除或校验 host-local Projection Locator。
- locator repair 不得修改 repo ledger facts、repo URL、repo display name 或 shadow branch identity。
- projection base 变更后，系统 **MUST** 先停止该 repo watcher，再执行 locator 更新、projection materialize / rebuild、watcher restart。
- projection base 变更不需要移动旧 workspace；旧目录只能作为外部数据源，经显式 import / repair 流程进入 pending 或 rebuild。
- repo rename / display name repair 不改变 projection base；但 workspace root 必须从 `<base>/<old_safe_repo_name>--<repo_id>/` realign / move 到 `<base>/<new_safe_repo_name>--<repo_id>/`。目标已存在、跨设备 move 不可安全完成、`.notegit` identity 不一致或目录冲突时必须 fail-closed。
- workspace root realign 前若存在 pending/staged/dirty workspace 或 projection fault，rename / display name repair **MUST** 先 fail-closed，并要求用户完成 commit、discard、repair 或显式 import。
- locator 缺失或冲突必须保持 `DegradedLocator`，直到用户显式提供可用 base 且计算出的 workspace root 可用。

### 7.5 Repo Rename Contract

- repo rename 是本地可写 repo 的 authority operation，必须通过 writer gate 与 `RepoId` admission。
- rename intent 必须携带 `repo_id`、`old_name`、`new_name` 与 `expected_name_epoch`。
- ledger append 前必须完成：selector -> `RepoId`、当前 `RepoNameBinding` 校验、safe name 规范化、目标 workspace root 预检、dirty/staged/pending/projection fault gate。
- ledger append 后，workspace realign、locator hint 更新、catalog hint 更新必须以同一个 `RepoId` 为锚点执行。
- 如果 ledger 已提交但 workspace realign 失败，该 repo 必须进入 `DegradedLocator` 或 `DegradedProjection`，并通过 repair runtime 暴露可恢复动作；不得把 rename 回滚为基于旧路径名的隐式绑定。

### 7.6 Catalog Conflict Repair

- 同名 display repo 但不同 logical identity 时，只允许修复 catalog/name hint drift，不得合并 authority。
- remote repo selector 若只能唯一解析到一个健康 remote repo，可做受控 fallback；一旦出现歧义，必须 fail-closed。

### 7.7 Startup Scan Contract

- startup materialize 遇到坏 repo 时，不得拖垮整个服务。
- 坏 repo 必须显式标记 degraded/quarantined。
- 被跳过的 repo 不得继续参与自动 scope 恢复。

## 8. Forbidden Patterns

- 直接用 `RepoName` 或 `Path` 驱动底层业务算子。
- 在 switcher / listing handler 里静默选择“第一个可用 repo”。
- 让 projection fallback 长期替代真正 repair。
- 让 metadata/path table 成为 rename/move/delete 的主写路径。
- 让 UI 直接根据名字推断 repo identity。
- 把 remote readonly repo 误暴露为可写 source。
- 让 repo name、URL、全局 vault root 或 `ledger_dir` 推断 projection base；repo name 只能作为 `RepoNameBinding` 的显示属性参与 `<safe_repo_name>--<repo_id>` workspace segment 派生。

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

### 9.4 View Layer

职责：

- 仅展示与发出切换意图
- 不得自行推断 repo authority

## 10. Refactor Target

长期应将 repo 逻辑显式收敛成三个独立 runtime：

- `repo_catalog_runtime`
- `projection_locator_runtime`
- `repo_scope_runtime`
- `projection_repair_runtime`

未来重构 **MUST** 收敛到这三个 runtime；`RepoManager`、CLI switcher handlers 与 `use_core` effects 不得共享隐式 repo scope 状态。

## 本章相关命令

- `P2P: Switch to Peer`
- `P2P: Establish Branch`

## 本章相关配置

- 无

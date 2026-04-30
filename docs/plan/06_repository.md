# 06_repository.md - 仓库与分支工程蓝图

## Metadata

- `Layer`: `Authority Core`
- `Status`: `Current MUST`
- `Counterpart Feature`: `docs/features/06_repository.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/07_storage_repo.md`
- `Primary Code Areas`: `crates/core/src/tree/`, `crates/core/src/ledger/manager/structure_projection*.rs`, `apps/cli/src/server/handlers/switcher*.rs`, `apps/web/src/hooks/use_core/callbacks_switch_*.rs`

## 1. Scope

本章定义 Repo、Branch、Tree Projection 与 Repo Health 的工程实现合同。

本章回答的问题只有两个：

1. 系统如何唯一识别并切换到某个 repo / branch / doc scope。
2. 结构树、仓库目录与 repo repair 如何在 authority 与 projection 之间分层。

用户可见行为、按钮文案与 Chrome MCP 手工路径属于 `docs/features/06_repository.md`，不属于本章。

## 2. Authoritative Entities

### 2.1 Repo Identity

- `RepoId` 是仓库权威身份，UUID-first。
- `RepoName`、`URL`、`Selector` 只允许作为输入别名或恢复线索。
- 任何进入业务算子的 repo 输入，在执行前 MUST 解析为 `RepoId`。

### 2.2 Branch Identity

- `Local Branch` = `ledger/local/*.redb` 中的本地 repo 事实集合。
- `Remote Branch` = `ledger/remotes/<PeerId>/*.redb` 中的远端镜像事实集合。
- Branch 是 writer identity 的作用域，不是任意命名的 git-style feature branch。

### 2.3 Tree Identity

- `NodeId` 是文件树节点的权威主键。
- `DocId` 是文件内容实体的权威主键。
- `Path`、`path_cache`、`TreeDelta`、`NodeMeta` 只能是 projection 或 projection cache。

### 2.4 Repo Health

每个 repo instance MUST 显式落入以下健康状态之一：

- `Healthy`
- `DegradedProjection`
- `DegradedCatalog`
- `Repairing`
- `Quarantined`

其中：

- `Healthy` 才允许正常 mounted write path。
- `Degraded*` 允许受控只读或 fallback 行为，但必须显式暴露给 runtime。
- `Quarantined` 表示该 repo 不再参与正常 scope 恢复、自动切换和默认列表绑定。

### 2.5 Selector Inputs and Logical Identity {#repo-selector-resolution-contract}

- repo 的逻辑身份基于 `RepoId`；`URL` 或其他 characteristic parameter 仅作为辅助识别线索。
- `RepoName` 相同但 `URL/RepoId` 不同的实例 MUST 视为完全不同的 repo。
- 后端接口 MAY 接受：
  - `RepoId`
  - `RepoName`
  - `URL`
  - `CurrentScopeFallback`
- 但进入任何底层 repo/document/source-control 算子前，必须解析成唯一 `RepoId`。
- selector 解析必须 UUID-first；`RepoName` 与 `URL` 只能辅助定位，不得覆盖已解析的 `RepoId`。
- selector 解析出现缺失、重复、metadata drift、URL 歧义时 MUST fail-closed。

逻辑 repo 归类规则：

- `URL / characteristic parameter` 匹配
  - 视为同一逻辑 repo 协作。
  - runtime 可以显示 shadow branches、remote mirrors、same-logical-repo peers。
- `URL / characteristic parameter` 不匹配
  - 视为不同逻辑 repo。
  - 应进入 multi-root workspace，而不是混入同一 repo 的 branch/scope。
- `Peer-only Repo`
  - 若只存在于远端且不匹配当前本地逻辑 repo，必须强制只读。
  - 仅允许 copy / inspect / diff / explicit import，禁止直接写入或错误绑定为 local writable repo。

## 3. Storage Layout

### 3.1 Physical Layout

- `ledger/local/<repo_name>.redb`
- `ledger/remotes/<peer_name>/<repo_name>.redb`
- `vault/<repo_name>/`
- `vault/<repo_name>/.notegit/`

### 3.2 Collision Rule

- 同一 branch 下，如果 `RepoName` 相同但 `RepoId/URL` 不同，系统 MUST 自动分配新的物理文件名。
- 物理文件名冲突的处理不得改变逻辑 repo identity。

### 3.3 Catalog Rule {#repo-catalog-contract}

- local repo catalog 与 remote repo catalog 是 selector / listing / switcher 的输入层，不是业务真值层。
- catalog 损坏时 MUST 进入 repair 或 fail-closed，不得静默把错误 repo 绑定到当前 scope。
- catalog entry 必须是可读 repo DB 文件，且 repo metadata 的 `RepoId / RepoName / URL` 不得相互漂移或重复。
- remote catalog 文件名冲突只能通过安全重命名或受控 repair 处理，不得合并不同 logical identity。

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
  -> Healthy
  -> DegradedProjection
  -> Repairing
  -> Healthy | Quarantined
```

约束：

- `ResolvingSelector` 必须先完成 selector -> `RepoId` 解析。
- `OpeningInstance` 必须验证 runtime tables、catalog、projection 依赖。
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
- fallback 生效时 repo health MUST 标记为 `DegradedProjection`。
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

- 树视图构建时 MUST 遵循：
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
  - 输出：remote repo bound in readonly mode
- `Remote -> Local`
  - SHOULD 优先恢复最近稳定本地 repo
  - 恢复失败时 MUST 回到 UUID-first 解析
- `Broken Persisted Scope -> Startup`
  - MUST 清理 stale last scope
  - MUST 重新 bootstrap 健康 repo 列表

## 7. Recovery / Repair Contract

### 7.1 Selector Recovery

- 如果用户提供 `RepoName`，系统 MAY 做别名解析。
- 如果解析结果不唯一或不一致，系统 MUST fail-closed。
- 从 `Remote -> Local` 返回时，系统 SHOULD 优先恢复最近一次稳定本地 repo。
- 最近本地 repo 不可解析时，必须回到严格 UUID-first 路径，而不是绑定任意本地 repo。

### 7.2 Catalog Repair {#repo-catalog-repair-contract}

- local / remote repo catalog repair 只能修复 catalog、name drift、blank selector、duplicate metadata。
- repair 不得修改 ledger authority 本身。
- repair 可以补全缺失 URL、重写漂移的显示名、分配安全物理文件名；但如果会合并两个 logical repo，必须 fail-closed。
- repair 后仍无法形成唯一 `RepoId / URL / filename` 映射时，repo 必须保持 degraded 或 quarantined。

### 7.3 Projection Repair

- structure projection 缺 parent、断链、脏 path cache 时，必须通过 rebuild / repair 处理。
- rebuild / repair 只允许重建 projection tables 与 workspace projection，不得修改 Structure Facts authority。
- 若 Structure Facts authority 本身引用缺失 parent / missing node / cycle / doc identity mismatch，repair MUST 输出结构化诊断并 fail-closed；该 repo 必须保持 `DegradedProjection` 或进入 quarantine，直到用户通过导出、重建 repo 或明确的 authority-level 迁移处理。
- repair 失败时 repo MUST 退出正常 mounted write path。

### 7.4 Catalog Conflict Repair

- 同名 repo 文件但不同 logical identity 时，只允许修复 catalog/name drift，不得合并 authority。
- remote repo selector 若只能唯一解析到一个健康 remote repo，可做受控 fallback；一旦出现歧义，必须 fail-closed。

### 7.5 Startup Scan Contract

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

## 9. Module Boundary

### 9.1 Authority Layer

- `crates/core/src/ledger`
- `crates/core/src/ledger/append_validate.rs`
- `crates/core/src/ledger/manager/ops_structure.rs`

职责：

- repo facts
- structure facts
- append validation

### 9.2 Projection / Repair Layer

- `crates/core/src/tree/`
- `crates/core/src/ledger/manager/structure_projection.rs`
- `crates/core/src/ledger/manager/structure_projection_support.rs`
- `crates/core/src/sync/materialize.rs`

职责：

- tree projection
- docs fallback
- structure repair
- startup materialize

### 9.3 Scope Runtime Layer {#repo-scope-runtime}

- `apps/cli/src/server/repo_scope*.rs`
- `apps/cli/src/server/session*.rs`
- `apps/cli/src/server/handlers/switcher*.rs`
- `apps/web/src/hooks/use_core/callbacks_switch_*.rs`
- `apps/web/src/hooks/use_core/effects/message_repo_scope*.rs`

职责：

- scope binding
- selector resolution
- last-local recovery
- stale scope cleanup

### 9.4 View Layer

- `apps/web/src/components/sidebar/repo_switcher.rs`
- `apps/web/src/components/branch_switcher/`
- `apps/web/src/components/sidebar/source_control/repositories.rs`

职责：

- 仅展示与发出切换意图
- 不得自行推断 repo authority

## 10. Code Mapping

- repo selector / resolution:
  - `apps/cli/src/server/handlers/switcher_selector.rs`
  - `apps/cli/src/server/handlers/switcher_requested_repo.rs`
  - `apps/cli/src/server/repo_scope_lookup.rs`
  - `crates/core/src/ledger/manager/locator.rs`
  - `crates/core/src/ledger/manager/repo_lookup.rs`
  - `crates/core/src/ledger/manager/remote_repo_select.rs`
- repo catalog / repair:
  - `crates/core/src/ledger/manager/repo_catalog_entries.rs`
  - `crates/core/src/ledger/manager/local_repo_metadata_repair.rs`
  - `crates/core/src/ledger/manager/remote_repo_scan.rs`
  - `crates/core/src/ledger/manager/remote_repo_scan_helpers.rs`
- branch / repo switching:
  - `apps/cli/src/server/handlers/switcher_branch.rs`
  - `apps/cli/src/server/handlers/switcher_repo.rs`
  - `apps/web/src/hooks/use_core/callbacks_switch_repo.rs`
  - `apps/web/src/hooks/use_core/callbacks_switch_branch.rs`
- session scope:
  - `apps/cli/src/server/session.rs`
  - `apps/cli/src/server/session_repo.rs`
  - `apps/cli/src/server/session_scope.rs`
- tree projection:
  - `crates/core/src/tree/manager.rs`
  - `crates/core/src/tree/delta.rs`
  - `crates/core/src/tree/from_docs.rs`
- repo repair:
  - `crates/core/src/ledger/manager/local_repo_metadata_repair.rs`
  - `crates/core/src/ledger/manager/local_repo_source_control_repair.rs`
  - `crates/core/src/ledger/manager/core_docs_fallback.rs`
  - `crates/core/src/sync/materialize.rs`

额外映射：

- tree state:
  - `crates/core/src/tree/manager.rs`
  - `crates/core/src/tree/delta.rs`
  - `crates/core/src/tree/from_docs.rs`
- scope recovery:
  - `apps/web/src/hooks/use_core/effects/message_protocol_control.rs`
  - `apps/web/src/hooks/use_core/effects/message_dispatch_protocol.rs`

## 11. Refactor Target

长期应将 repo 逻辑显式收敛成三个独立 runtime：

- `repo_catalog_runtime`
- `repo_scope_runtime`
- `projection_repair_runtime`

未来重构 **MUST** 朝这三个 runtime 收敛，避免让 `RepoManager`、CLI switcher handlers 与 `use_core` effects 继续共享隐式 repo scope 状态。

## 本章相关命令

- `P2P: Switch to Peer`
- `P2P: Establish Branch`

## 本章相关配置

- 无

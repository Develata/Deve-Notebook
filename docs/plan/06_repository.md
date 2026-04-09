# 06_repository.md - 仓库工程蓝图

本章只定义 repo / branch / tree / repo health 的工程实现，不描述用户界面工作流。功能语义见 [../features/06_repository.md](../features/06_repository.md)，自动化验收见 [../acceptance-cases/07_storage_repo.md](../acceptance-cases/07_storage_repo.md)。

## 1. 目标

- 所有 repo 操作都必须先解析到稳定身份，再执行底层算子。
- tree 是 structure projection，不是权威写源。
- remote / spectator 必须天然只读。

## 2. 权威实体

- `RepoId`
  - 仓库唯一身份，底层执行主键。
- `BranchScope`
  - `Local` 或 `Remote(PeerId)`。
- `NodeId / DocId`
  - 结构与文档事实主键。
- `Structure Facts`
  - `Create / Rename / Move / Delete` 的唯一权威来源。
- `RepoHealth`
  - `Healthy / Degraded / Quarantined / Repairing`。

## 3. 分层

### 3.1 Authority

- repo、doc、node 的真值在 ledger facts 中。
- 任何底层读写都必须先完成 `UUID-first` 解析。

### 3.2 Projection

- tree manager 是 structure projection 的内存视图。
- `path_cache` 只是 projection cache，不得作为业务主键。

### 3.3 Runtime

- repo switch、branch switch、last-local-repo recovery 属于 scope runtime。
- UI 只能请求切换，不能直接篡改 repo binding。

## 4. 状态机

### 4.1 Repo 健康状态

- `Healthy`
- `Degraded`
- `Quarantined`
- `Repairing`

### 4.2 激活状态

- `LocalActive`
- `RemoteActive`
- `RecoveringLocal`
- `Unavailable`

### 4.3 转换规则

- `SelectLocalRepo -> LocalActive`
- `SelectRemoteBranch -> RemoteActive`
- `LeaveRemote -> RecoveringLocal`
- `RecoverLastStableLocal -> LocalActive`
- `ProjectionBroken -> Degraded`
- `RepairFailed -> Quarantined`

## 5. 写入合同

- `CreateDoc / RenameDoc / MoveDoc / DeleteDoc`
  - 最终必须以 `NodeId / DocId` 进入 ledger。
- tree/path metadata 不得绕过 structure facts 直接写入。
- spectator / remote branch 下所有结构写操作必须 fail-closed。

## 6. 恢复与修复合同

- 启动期如果 projection 失效，可暂时降级，但不得把 projection 当 authority 覆盖回 ledger。
- 历史坏 repo 必须走受控 repair 或 quarantine。
- 从 remote 返回 local 时，应优先恢复最近稳定本地 repo；若不可恢复，必须显式走 fail-closed fallback。

## 7. Remote / Spectator 边界

- remote branch 是只读镜像视图，不是可写 branch。
- 允许 read / diff / export / merge-into-local。
- 禁止直接写回 remote ledger。

## 8. 禁止事项

- 禁止 name/path-only 直接驱动底层 repo/file 算子。
- 禁止把 tree projection 当作结构权威。
- 禁止跨 peer 直接写入别人的 branch。
- 禁止 repo 损坏时静默绑定到其它 repo。

## 9. 代码边界

- `crates/core/src/ledger/`
  - repo facts、structure facts、append validation。
- `crates/core/src/tree/`
  - tree projection。
- `apps/cli/src/server/handlers/switcher*`
  - repo / branch selection runtime。
- `apps/web/src/hooks/use_core/`
  - current repo scope、last local repo hint、readonly boundary。

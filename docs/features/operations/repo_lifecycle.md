# repo_lifecycle.md - Host-owned Repo Lifecycle 操作流

## Metadata

- `Flow ID`: `flow.repo.lifecycle`
- `Domain`: `repository`
- `Related Feature Chapters`: `docs/features/06_repository.md`
- `Related Acceptance Cases`: `STORE-014A`

## Operations

### `op.repo.lifecycle.submit-create`

- `Name`: `Submit Create Job`
- `Surface`: `typed-ws`
- `Preconditions`: authenticated；request_id 新建或与既有 normalized intent 一致
- `Immediate Result`: host runtime 返回 job_id/target RepoId；transport waiter 不拥有 job
- `Application Entry`: `apps/web/src/runtime/repo_control_client.rs` → `apps/cli/src/server/handlers/repo_control.rs` → `apps/cli/src/server/runtime/repo_lifecycle_job_runtime.rs`

### `op.repo.lifecycle.prepare-remove`

- `Name`: `Prepare Local Removal`
- `Surface`: `typed-ws`
- `Preconditions`: authenticated exact RepoId/scope；ownership与owner plans可读取；optional fallback必须由用户显式选择；不得改变membership
- `Immediate Result`: backend返回safe preview、preparation_id、optional opaque fallback binding与五分钟一次性token；不删除对象
- `Application Entry`: `apps/web/src/runtime/repo_control_client.rs` → `apps/cli/src/server/handlers/repo_control.rs` → `apps/cli/src/server/runtime/repo_lifecycle_job_runtime.rs`

### `op.repo.lifecycle.execute-remove`

- `Name`: `Execute Prepared Local Removal`
- `Surface`: `typed-ws`
- `Preconditions`: exact preparation/token/issuer/scope；per-repo single-flight可用；owner plans仍exact
- `Immediate Result`: token consumption与job admission原子持久化，host runtime接管job；observer绑定当前connection/scope epoch
- `Application Entry`: `apps/web/src/runtime/repo_control_client.rs` → `apps/cli/src/server/handlers/repo_control.rs` → `apps/cli/src/server/runtime/repo_lifecycle_job_runtime.rs`

### `op.repo.lifecycle.query`

- `Name`: `Query Lifecycle Completion`
- `Surface`: `typed-ws`
- `Preconditions`: authenticated；request_id 已登记
- `Immediate Result`: 返回 Accepted/Running/terminal/repair typed status
- `Application Entry`: `apps/cli/src/server/handlers/repo_control.rs` → `apps/cli/src/server/runtime/repo_lifecycle_job_runtime.rs`

### `op.repo.lifecycle.publish`

- `Name`: `Publish Settled Lifecycle Outcome`
- `Surface`: `backend-coordination`
- `Preconditions`: immutable committed-cut plan 已完成 settlement
- `Immediate Result`: host-owned publication sink 更新 repo list，并对仍 exact 的 session observer条件应用 scope outcome
- `Application Entry`: `apps/cli/src/server/runtime/repo_lifecycle_job_runtime/host.rs` → `apps/cli/src/server/runtime/repo_session_runtime.rs`

## Notes

- create使用prepare → short authority cut → settle；remove显式执行Prepare → Execute，Execute先原子持久化token consumption与job admission，再按provider quiesce → watcher E2 → authority retirement → short membership cut → owner cleanup收敛。create的长期membership authority是per-RepoId catalog record；remove先把它原子切为transient`Removed` tombstone，再按immutable ownership manifest经各owner清理Deve-owned repo state，全部收敛后删除tombstone。最后一个repo允许删除并进入`NoScope`。remove永不删除workspace root、Markdown/附件、`.git`、remote shadows、persistent authority lock pathname或位于removal roots之外的operator恢复输入。
- alias set 不属于 lifecycle；handler/connection drop 不取消已 admission job。

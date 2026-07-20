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

### `op.repo.lifecycle.submit-remove`

- `Name`: `Submit Remove Job`
- `Surface`: `typed-ws`
- `Preconditions`: exact RepoId；Remote Import/fallback admission 可证明；per-repo single-flight 可用
- `Immediate Result`: host runtime 接管 job；observer 绑定当前 connection/scope epoch
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

- create/remove 使用 prepare → short authority cut → settle。create 的长期 membership authority 是 per-RepoId catalog record；remove 先把它原子切为 transient `Removed` tombstone，再按 immutable ownership manifest 清理 Deve-owned repo state，全部收敛后删除 tombstone。remove 永不删除 workspace root、Markdown/附件、`.git`、remote shadows 或显式 backup/export。
- alias set 不属于 lifecycle；handler/connection drop 不取消已 admission job。

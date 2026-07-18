# repo_lifecycle.md - Host-owned Repo Lifecycle 操作流

## Metadata

- `Flow ID`: `flow.repo.lifecycle`
- `Domain`: `repository`
- `Related Feature Chapters`: `docs/features/06_repository.md`
- `Related Acceptance Cases`: `REPO-FEAT-03`

## Operations

### `op.repo.lifecycle.submit-create`

- `Name`: `Submit Create Job`
- `Surface`: `typed-ws`
- `Preconditions`: authenticated；request_id 新建或与既有 normalized intent 一致
- `Immediate Result`: host runtime 返回 job_id/target RepoId；transport waiter 不拥有 job
- `Application Entry`: `planned/no-code-yet`

### `op.repo.lifecycle.submit-remove`

- `Name`: `Submit Remove Job`
- `Surface`: `typed-ws`
- `Preconditions`: exact RepoId；Remote Import/fallback admission 可证明；per-repo single-flight 可用
- `Immediate Result`: host runtime 接管 job；observer 绑定当前 connection/scope epoch
- `Application Entry`: `planned/no-code-yet`

### `op.repo.lifecycle.query`

- `Name`: `Query Lifecycle Completion`
- `Surface`: `typed-ws`
- `Preconditions`: authenticated；request_id 已登记
- `Immediate Result`: 返回 Accepted/Running/terminal/repair typed status
- `Application Entry`: `planned/no-code-yet`

### `op.repo.lifecycle.publish`

- `Name`: `Publish Settled Lifecycle Outcome`
- `Surface`: `backend-coordination`
- `Preconditions`: immutable committed-cut plan 已完成 settlement
- `Immediate Result`: host-owned publication sink 更新 repo list，并对仍 exact 的 session observer条件应用 scope outcome
- `Application Entry`: `planned/no-code-yet`

## Notes

- create/remove 使用 prepare → short authority cut → settle；唯一 committed fact 是 per-RepoId catalog record。
- alias set 不属于 lifecycle；handler/connection drop 不取消已 admission job。

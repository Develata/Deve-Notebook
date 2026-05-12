# repo_switch.md - 仓库切换操作流示例

## Metadata

- `Flow ID`: `flow.repo.switch`
- `Domain`: `repository`
- `Related Feature Chapters`: `docs/features/06_repository.md`
- `Related Acceptance Cases`: `REPO-FEAT-01`, `REPO-FEAT-03`

## Operations

### `op.repo.switch.open-switcher`

- `Name`: `Open Repo Switcher`
- `Surface`: `sidebar`
- `Trigger`: 点击 repo switcher 触发按钮
- `Preconditions`: 主界面已加载
- `Immediate Result`: repo 下拉菜单打开
- `Application Entry`: `apps/web/src/components/sidebar/repo_switcher.rs`

### `op.repo.switch.choose-repo`

- `Name`: `Choose Repo Target`
- `Surface`: `sidebar`
- `Trigger`: 用户点击某个 repo 名称
- `Preconditions`: switcher 已打开，repo list 已可用
- `Immediate Result`: 准备进入 repo scope 切换
- `Application Entry`: `apps/web/src/components/sidebar/repo_switcher.rs`, `apps/web/src/hooks/use_core/callbacks_switch/repo.rs`

### `op.repo.switch.request-switch`

- `Name`: `Request Repo Switch`
- `Surface`: `ui-state`
- `Trigger`: 前端发送 `ClientMessage::SwitchRepo`
- `Preconditions`: 当前没有 pending scope switch，目标 repo 不是当前 repo
- `Immediate Result`: 设置 `pending_repo_switch` 与 `pending_repo_switch_nonce`
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks_switch/repo.rs`, `apps/cli/src/server/handlers/switcher/switcher_repo.rs`

### `op.repo.switch.receive-scope`

- `Name`: `Receive Repo Scope Rebind`
- `Surface`: `ui-state`
- `Trigger`: 服务端返回 `RepoSwitched`、doc list、tree update
- `Preconditions`: `op.repo.switch.request-switch` 已执行
- `Immediate Result`: 当前 repo / scope_nonce / tree / docs 全部重绑到新 repo
- `Application Entry`: `apps/cli/src/server/handlers/switcher/switcher_payload.rs`, `apps/web/src/hooks/use_core/effects/message_repo_scope/mod.rs`

## Notes

- repo switch 的关键不是菜单本身，而是 `RepoName -> RepoId` 解析、`switch_nonce`、以及整套 repo view preload。
- 这条 flow 与 `branch-switch` 共享 switcher 家族，但 authority 目标不同：这里重绑的是 repo scope 本身。

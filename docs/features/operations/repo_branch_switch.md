# repo_branch_switch.md - 分支切换操作流示例

## Metadata

- `Flow ID`: `flow.repo.branch-switch`
- `Domain`: `repository`
- `Related Feature Chapters`: `docs/features/06_repository.md`, `docs/features/12_commands.md`
- `Related Acceptance Cases`: `CMD-004`, `REPO-FEAT-02`, `REPO-FEAT-03`

## Operations

### `op.repo.branch-switch.open-switcher`

- `Name`: `Open Branch Switcher`
- `Surface`: `keyboard-shortcut`
- `Trigger`: `Ctrl/Cmd+Shift+K`
- `Preconditions`: 应用主界面已加载，repo scope 已建立
- `Immediate Result`: branch switch UI 打开
- `Application Entry`: `apps/web/src/components/search_box/`, `apps/web/src/components/branch_switcher/`

### `op.repo.branch-switch.choose-branch`

- `Name`: `Choose Branch Target`
- `Surface`: `keyboard-or-pointer`
- `Trigger`: 选择本地或远端 branch 候选项
- `Preconditions`: switcher 已打开，候选列表非空
- `Immediate Result`: 目标 branch 被选中
- `Application Entry`: `apps/web/src/components/search_box/providers_branch.rs`, `apps/web/src/components/search_box/logic/execute.rs`

### `op.repo.branch-switch.request-switch`

- `Name`: `Request Branch Switch`
- `Surface`: `ui-state`
- `Trigger`: 调用 `on_switch_branch`
- `Preconditions`: `switch_nonce` 严格大于当前 `scope_nonce`
- `Immediate Result`: 发出 `ClientMessage::SwitchBranch`
- `Application Entry`: `apps/web/src/hooks/use_core/`, `crates/core/src/protocol/client.rs`, `apps/cli/src/server/ws/route/core.rs`

### `op.repo.branch-switch.receive-switch-result`

- `Name`: `Receive Branch Switch Result`
- `Surface`: `ui-state`
- `Trigger`: 服务端返回 `BranchSwitched` 或 `ProtocolError`
- `Preconditions`: `op.repo.branch-switch.request-switch` 已执行
- `Immediate Result`: 当前 branch / scope 更新，或明确进入 stale-scope 错误
- `Application Entry`: `crates/core/src/protocol/server.rs`, `apps/web/src/api/incoming/`, `apps/cli/src/server/handlers/switcher/`

## Notes

- 本 flow 的核心不是打开切换器，而是 `switch_nonce` 与 `scope_nonce` 的严格门控。
- branch switch 会重绑 repo-scoped session，因此属于仓库架构核心流，而不是单纯 UI 导航。

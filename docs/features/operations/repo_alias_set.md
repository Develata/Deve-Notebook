# repo_alias_set.md - Host-local Repo Alias 操作流

## Metadata

- `Flow ID`: `flow.repo.alias-set`
- `Domain`: `repository`
- `Related Feature Chapters`: `docs/features/06_repository.md`
- `Related Acceptance Cases`: `STORE-014A`

## Operations

### `op.repo.alias-set.open`

- `Name`: `Open Local Alias Editor`
- `Surface`: `sidebar`
- `Preconditions`: authenticated；backend RepoList 已返回 exact RepoId/alias revision
- `Immediate Result`: 只打开本地表单，不改变 repo scope、workspace 或 authority
- `Application Entry`: `apps/web/src/components/sidebar/repo_switcher/row/rename_form.rs`

### `op.repo.alias-set.submit`

- `Name`: `Submit Alias CAS`
- `Surface`: `typed-ws`
- `Preconditions`: exact RepoId；alias 合法；expected alias revision 来自 backend projection
- `Immediate Result`: `RepoControlRequest::SetAlias`
- `Application Entry`: `apps/web/src/runtime/repo_control_client.rs` → `apps/cli/src/server/handlers/repo_control.rs`

### `op.repo.alias-set.render-result`

- `Name`: `Render Alias Result`
- `Surface`: `sidebar`
- `Preconditions`: request_id 匹配；typed ack/error
- `Immediate Result`: client 只接纳 exact request/scope 的 typed ack/error；alias projection 由 backend broadcast 的 exact RepoList 更新；不解析 detail
- `Application Entry`: `apps/web/src/runtime/repo_control_client.rs` → `apps/web/src/hooks/use_core/effects/message_dispatch_route_control.rs`

## Notes

- alias 是 host-local display state；本 flow 不触发 watcher、workspace move、Ledger、sync 或 scope switch。
- duplicate alias 可显示，但任何 alias-only selector 歧义必须 fail-closed；产品 intent 始终携带 exact RepoId。

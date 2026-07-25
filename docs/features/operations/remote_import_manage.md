# remote_import_manage.md - Remote Import Session 管理操作流

## Metadata

- `Flow ID`: `flow.remote-import.manage`
- `Domain`: `remote-import`
- `Related Feature Chapters`: `docs/features/06_repository.md`, `docs/features/12_commands.md`
- `Related Acceptance Cases`: `STORE-023`

## Operations

### `op.remote-import.manage.refresh`

- `Name`: `Refresh Sealed Candidate`
- `Surface`: `remote-import`
- `Trigger`: 用户点击 Refresh
- `Preconditions`: exact session/revision；sealed blobs 可验证；RepoId/branch/source/locator binding 未变化
- `Immediate Result`: backend 只从 sealed blobs 重算新 candidate revision，并绑定当前 Ledger head 与 ignore snapshot
- `Application Entry`: typed WS / `apps/cli/src/commands/remote_import/` → `apps/cli/src/remote_import_runtime.rs` → `crates/core/src/remote_import/`

### `op.remote-import.manage.discard`

- `Name`: `Discard Remote Import Session`
- `Surface`: `remote-import`
- `Trigger`: 用户确认 Discard
- `Preconditions`: exact active/terminal session；session CAS 成功
- `Immediate Result`: session 转为 Discarded，并显式进入/完成 cleanup lifecycle
- `Application Entry`: typed WS / `apps/cli/src/commands/remote_import/` → `apps/cli/src/remote_import_runtime.rs` → `crates/core/src/remote_import/`

### `op.remote-import.manage.repair-dry-run`

- `Name`: `Inspect Remote Import Cleanup`
- `Surface`: `cli`
- `Trigger`: operator 运行 remote-import repair
- `Preconditions`: repo/session store 与 host artifact root 可读
- `Immediate Result`: 只输出 typed cleanup plan，不删除 artifact 或改变 authority
- `Application Entry`: `apps/cli/src/commands/remote_import/` → `apps/cli/src/remote_import_runtime.rs` → `crates/core/src/remote_import/`

### `op.remote-import.manage.repair-apply`

- `Name`: `Apply Remote Import Cleanup`
- `Surface`: `cli`
- `Trigger`: operator 显式运行 remote-import repair --apply
- `Preconditions`: dry-run plan 仍 exact、目标 path containment 重验、显式 destructive intent
- `Immediate Result`: 只清理已证明属于 terminal/cleanup_pending record 的 host artifact，并持久化 cleanup outcome
- `Application Entry`: `apps/cli/src/commands/remote_import/` → `apps/cli/src/remote_import_runtime.rs` → `crates/core/src/remote_import/`

## Response Flow

1. exact revalidate session/revision 与 artifact identity。
2. Refresh 只允许 head/ignore 基线重绑；source/locator/branch/membership/tamper drift 不可重绑。Refresh/Discard/repair 各自通过 typed CAS 收敛；新 remote content 必须 Discard 后重新 Prepare。
3. cleanup failure 保留 cleanup_pending，不自动裁剪；Applied receipt 仍为 projection outcome=`Pending`
   时 repair 只报告不可执行 debt，保留 sealed recovery artifacts，不能删除。

## Notes

- B4产品Refresh/Discard/Repair与现有RepoId/provider lifecycle coordination已激活；B5 Web
  Refresh/Discard management只发送typed intent；R4已实现authority Quiescing前exact封存immutable
  owner plan，以及Removed cut后按该plan执行artifact-only cleanup。B6 已为 STORE-023 登记真实 browser
  producer；最终 candidate current-HEAD receipt仍须重跑。

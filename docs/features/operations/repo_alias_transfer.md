# repo_alias_transfer.md - Repo Alias JSON Transfer 操作流

## Metadata

- `Flow ID`: `flow.repo.alias-transfer`
- `Domain`: `repository`
- `Related Feature Chapters`: `docs/features/06_repository.md`, `docs/features/12_commands.md`
- `Related Acceptance Cases`: `CMD-008`

## Operations

### `op.repo.alias-transfer.export`

- `Name`: `Export Host Alias Map`
- `Surface`: `cli`
- `Preconditions`: local catalog/alias store 可读
- `Immediate Result`: 按 RepoId 排序的 deterministic JSON v1，只含 explicit alias binding；不含 revision/path/peer/secret；输出 atomic no-clobber，拒绝 Ledger tree
- `Application Entry`: `apps/cli/src/commands/repo_alias.rs` → `crates/core/src/ledger/manager/host_repo_alias.rs`

### `op.repo.alias-transfer.preview-import`

- `Name`: `Preview Alias Import`
- `Surface`: `cli`
- `Preconditions`: bounded JSON v1 顶层合法
- `Immediate Result`: 完整 accepted/warning summary；不写 store
- `Application Entry`: `apps/cli/src/commands/repo_alias.rs` → `crates/core/src/ledger/manager/host_repo_alias/import.rs`

### `op.repo.alias-transfer.apply-import`

- `Name`: `Apply Alias Import Batch`
- `Surface`: `cli`
- `Preconditions`: 显式 `--apply`；apply-time membership/store 重校验
- `Immediate Result`: valid entries 单批原子写入；unknown/invalid/duplicate entry warning+skip；store failure 全局报错
- `Application Entry`: `apps/cli/src/commands/repo_alias.rs` → `crates/core/src/ledger/manager/host_repo_alias/store.rs`

## Notes

- Web 不实现本 flow；CLI 只调用 backend/core typed runtime，不复制 JSON admission 规则。
- inter-peer sync、Remote Projection 与 Remote Import 不读取该文件。
- Runtime/CLI 已实现；tag-ready producer receipt 尚未封存，因此 acceptance matrix 继续保留 evidence gap，不能把代码测试冒充 freshness receipt。

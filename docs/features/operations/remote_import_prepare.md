# remote_import_prepare.md - Remote Import Prepare 操作流

## Metadata

- `Flow ID`: `flow.remote-import.prepare`
- `Domain`: `remote-import`
- `Related Feature Chapters`: `docs/features/06_repository.md`, `docs/features/12_commands.md`
- `Related Acceptance Cases`: `STORE-019`

## Operations

### `op.remote-import.prepare.webdav`

- `Name`: `Prepare WebDAV Remote Import`
- `Surface`: `remote-import`
- `Trigger`: 用户选择 WebDAV Prepare
- `Preconditions`: authenticated、Ledger 可读、无 active session、provider/profile admitted
- `Immediate Result`: backend 捕获 bounded immutable manifest/blobs 并发布 Ready session；不写 workspace 或 Ledger facts
- `Application Entry`: `apps/cli/src/remote_projection_transport/webdav/source.rs` → `apps/cli/src/remote_import_runtime.rs` → `crates/core/src/remote_import/`

### `op.remote-import.prepare.s3`

- `Name`: `Prepare S3 Remote Import`
- `Surface`: `remote-import`
- `Trigger`: 用户选择 S3 Prepare
- `Preconditions`: authenticated、Ledger 可读、无 active session、S3/S3-compatible profile exact admission 通过
- `Immediate Result`: backend 捕获 bounded immutable manifest/blobs 并发布 Ready session；不写 workspace 或 Ledger facts
- `Application Entry`: `apps/cli/src/remote_projection_transport/s3/source.rs` → `apps/cli/src/remote_import_runtime.rs` → `crates/core/src/remote_import/`

## Response Flow

1. Reserve durable Preparing。
2. Provider 向 bounded sink ordered streaming，sink 验证 path/budget/digest。
3. 原子发布 blobs/manifest/candidate，CAS 为 Ready；失败保持 typed terminal evidence。

## Notes

- B4 已接入 provider-bound product Prepare 并删除旧 pull；STORE-019 只保留 B6 fresh receipt 的真实 gap。

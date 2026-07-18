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
- `Application Entry`: B2 已落地 `apps/cli/src/remote_projection_transport/` 的 ordered source-acquisition adapter；接入 `remote_import_runtime` 的产品 Prepare 仍待 B4，旧 pull 仅是隔离在 `remote_projection_legacy` 的过渡 carrier

### `op.remote-import.prepare.s3`

- `Name`: `Prepare S3 Remote Import`
- `Surface`: `remote-import`
- `Trigger`: 用户选择 S3 Prepare
- `Preconditions`: authenticated、Ledger 可读、无 active session、S3/S3-compatible profile exact admission 通过
- `Immediate Result`: backend 捕获 bounded immutable manifest/blobs 并发布 Ready session；不写 workspace 或 Ledger facts
- `Application Entry`: B2 已落地 `apps/cli/src/remote_projection_transport/` 的 ordered source-acquisition adapter；接入 `remote_import_runtime` 的产品 Prepare 仍待 B4，旧 pull 仅是隔离在 `remote_projection_legacy` 的过渡 carrier

## Response Flow

1. Reserve durable Preparing。
2. Provider 向 bounded sink ordered streaming，sink 验证 path/budget/digest。
3. 原子发布 blobs/manifest/candidate，CAS 为 Ready；失败保持 typed terminal evidence。

## Notes

- B2 source acquisition 已有 transport-level producer；STORE-019 仍保持产品 Prepare 与 B6 fresh receipt 的真实 gap，旧 pull test 不是目标证据。

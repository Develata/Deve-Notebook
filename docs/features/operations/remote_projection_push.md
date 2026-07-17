# remote_projection_push.md - Remote Projection Push 操作流

## Metadata

- `Flow ID`: `flow.remote-projection.push`
- `Domain`: `remote-projection`
- `Related Feature Chapters`: `docs/features/06_repository.md`, `docs/features/12_commands.md`
- `Related Acceptance Cases`: `STORE-018`

## Operations

### `op.remote-projection.push.webdav`

- `Name`: `Push Projection through WebDAV`
- `Surface`: `cli-or-command-palette`
- `Trigger`: 用户选择 WebDAV push
- `Preconditions`: local repo 为 Healthy + Mounted，locator/profile 与 identity gate 通过
- `Immediate Result`: backend 只把当前 Markdown Projection Workspace streaming 到 admitted WebDAV target
- `Application Entry`: 当前 carrier 为 `apps/cli/src/commands/projection_remote/`；B2 将 transport 抽到共享 host infra

### `op.remote-projection.push.s3`

- `Name`: `Push Projection through S3`
- `Surface`: `cli-or-command-palette`
- `Trigger`: 用户选择 S3 push
- `Preconditions`: local repo 为 Healthy + Mounted，S3/S3-compatible profile exact admission 通过
- `Immediate Result`: backend 只把当前 Markdown Projection Workspace streaming 到 admitted S3 target
- `Application Entry`: 当前 carrier 为 `apps/cli/src/commands/projection_remote/`；B2 将 transport 抽到共享 host infra

## Response Flow

1. Surface 发送 provider-specific typed push intent。
2. Host transport 解析 locator/profile、验证 repo/workspace identity、逐文件上传。
3. 返回 typed push report；provider partial failure 不改 Ledger、Source Control 或 workspace。

## Notes

- B0 批准 target；当前 push transport 仍位于 CLI command subtree，是 B2 active drift。
- Push 与 Remote Import source acquisition 共享 adapter infrastructure，不共享业务 interface。

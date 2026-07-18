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
- `Application Entry`: `apps/cli/src/remote_projection_transport/`；`apps/cli/src/commands/projection_remote.rs` 仅保留 CLI grammar、诊断与调用壳

### `op.remote-projection.push.s3`

- `Name`: `Push Projection through S3`
- `Surface`: `cli-or-command-palette`
- `Trigger`: 用户选择 S3 push
- `Preconditions`: local repo 为 Healthy + Mounted，S3/S3-compatible profile exact admission 通过
- `Immediate Result`: backend 只把当前 Markdown Projection Workspace streaming 到 admitted S3 target
- `Application Entry`: `apps/cli/src/remote_projection_transport/`；`apps/cli/src/commands/projection_remote.rs` 仅保留 CLI grammar、诊断与调用壳

## Response Flow

1. Surface 发送 provider-specific typed push intent。
2. Online host 通过 `WatcherRuntimeView`，standalone CLI 通过临时 owned `RepoWatcherHandle` 验证 mount readiness；随后验证 workspace identity，transport 只解析 locator/profile 并逐文件上传。
3. 返回 typed push report；provider partial failure 不改 Ledger、Source Control 或 workspace。

## Notes

- B2 已把 provider/profile/credential/HTTP/signing、push 与 ordered source acquisition 收口到共享 host transport；push flow 不再是 active drift。
- Standalone push 的临时 watcher 只建立本次 host operation 的 ingestion readiness；W6 继续负责统一 E2 final-state shutdown，不得把 deterministic enumeration 描述成 point-in-time snapshot。
- Push 与 Remote Import source acquisition 共享 adapter infrastructure，不共享业务 interface。

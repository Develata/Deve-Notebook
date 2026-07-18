# remote_import_review.md - Remote Import Review 操作流

## Metadata

- `Flow ID`: `flow.remote-import.review`
- `Domain`: `remote-import`
- `Related Feature Chapters`: `docs/features/06_repository.md`, `docs/features/07_diff_logic.md`
- `Related Acceptance Cases`: `STORE-020`

## Operations

### `op.remote-import.review.list`

- `Name`: `List Remote Import Sessions`
- `Surface`: `remote-import`
- `Trigger`: 打开 Remote Import view
- `Preconditions`: repo/branch/scope exact，Ledger/session store 可读
- `Immediate Result`: backend 返回 typed session summaries
- `Application Entry`: `apps/cli/src/commands/remote_import/` / typed WS → `apps/cli/src/remote_import_runtime.rs` → `crates/core/src/remote_import/`；B5 补独立 `remote_import_client`

### `op.remote-import.review.show`

- `Name`: `Show Remote Import Session`
- `Surface`: `remote-import`
- `Trigger`: 打开 exact session/revision
- `Preconditions`: exact session/revision 存在
- `Immediate Result`: backend 返回 state、summary 与 typed blockers
- `Application Entry`: `apps/cli/src/commands/remote_import/` / typed WS → `apps/cli/src/remote_import_runtime.rs` → `crates/core/src/remote_import/`；B5 补独立 `remote_import_client`

### `op.remote-import.review.page`

- `Name`: `Page Remote Import Candidate`
- `Surface`: `remote-import`
- `Trigger`: 请求下一页 candidate entries
- `Preconditions`: cursor opaque、绑定相同 revision，limit ≤ 200
- `Immediate Result`: backend 返回 entry_id、display label、change kind 与 typed blocker
- `Application Entry`: typed WS → `apps/cli/src/remote_import_runtime.rs` → `crates/core/src/remote_import/`；B5 补独立 `remote_import_client`

### `op.remote-import.review.diff`

- `Name`: `Diff Remote Import Entry`
- `Surface`: `remote-import`
- `Trigger`: 用户点击 candidate row
- `Preconditions`: opaque strong entry_id 属于 exact session/revision
- `Immediate Result`: backend 返回 typed diff，不暴露 host/provider/blob path 或 digest
- `Application Entry`: `apps/cli/src/commands/remote_import/` / typed WS → `apps/cli/src/remote_import_runtime.rs` → backend typed diff；只允许复用无状态 diff primitive

## Response Flow

1. List request 绑定 `request_id + repo/branch/scope`；Show/Page/Diff 还绑定 exact session/revision。
2. Backend exact revalidate 后返回自带 session/revision 的 summaries，或分页并生成 typed diff/blocker。
3. Web 先校验 request/scope；已选择 session 的 response 再校验 session/revision。任一不匹配即丢弃，
   只渲染 backend projection。

## Notes

- 无 checkbox、逐文件选择或前端 blocker 推理。
- B4 backend/CLI review 已激活且旧 External Changes/Source Control substitute 已删除；B5 独立 thin client 与 B6 producer receipt 仍是明确缺口。

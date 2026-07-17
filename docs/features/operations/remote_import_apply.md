# remote_import_apply.md - Remote Import Apply 操作流

## Metadata

- `Flow ID`: `flow.remote-import.apply`
- `Domain`: `remote-import`
- `Related Feature Chapters`: `docs/features/04_storage.md`, `docs/features/06_repository.md`
- `Related Acceptance Cases`: `STORE-021`

## Operations

### `op.remote-import.apply.session`

- `Name`: `Apply Entire Remote Import Session`
- `Surface`: `remote-import`
- `Trigger`: 用户确认 Apply
- `Preconditions`: exact repo/branch/scope/session/revision，Healthy + Mounted，writer gate，零 blocker
- `Immediate Result`: sealed source-specific batch 在单一 Redb transaction 提交全部 upsert facts/receipt；commit 后才进行 Projection writeback
- `Application Entry`: target 为逻辑 `remote_import_runtime -> sealed authority writer`；当前 `crates/core/src/source_control/external_apply.rs` 只是 B3 抽取输入，不是 Remote Import authority

## Response Flow

1. Server exact revalidate session、head、digests、writer、locator/ignore 与 overlap。
2. 一个 transaction提交全部 facts/indexes、Applied receipt immutable core + projection outcome Pending、clear active 与 cleanup_pending。
3. transaction 后幂等执行 Projection writeback；第二个短 Redb transaction 把 outcome CAS 为 Written，或与 durable projection fault 一起 CAS 为 Degraded。崩溃/重试从 Pending 恢复，不再 append。

## Notes

- B0 为 `planned/no-code-yet`；STORE-021 是 first-tag `remote-import` journey 的真实 gap。
- 响应丢失后，相同请求返回 stored receipt，不重复 append。
- Pending response 必须明确表示 Ledger 已提交且 Projection recovery 未完成，不能伪装成未提交。

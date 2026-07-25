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
- `Preconditions`: fresh Apply 需要 exact repo/branch/scope/session/revision、Healthy + Mounted、writer gate、零 blocker；已提交 receipt 的 exact replay 保留 identity + Mounted gate，但不重新执行 health/provider admission
- `Immediate Result`: sealed source-specific batch 在单一 Redb transaction 提交全部 upsert facts/receipt；commit 后才进行 Projection writeback
- `Application Entry`: typed WS / `apps/cli/src/commands/remote_import/` → `apps/cli/src/remote_import_runtime.rs` → `crates/core/src/remote_import/` → `crates/core/src/ledger/manager/prepared_change_batch/remote_import.rs`

## Response Flow

1. Server exact revalidate session、head、digests、writer、locator/ignore 与 overlap。
2. 一个 transaction提交全部 facts/indexes、Applied receipt immutable core + projection outcome Pending、clear active 与 cleanup_pending。
3. transaction 后幂等执行 Projection writeback；第二个短 Redb transaction 把 outcome CAS 为 Written，或与同 repo Redb v4 `PROJECTION_FAULTS` typed evidence 一起 CAS 为 Degraded。崩溃/重试从 Pending 恢复，不再 append。

## Notes

- B4 已接入 Mounted product gate、current locator/ignore admission、Ledger-to-Projection
  rematerialization 与 typed product intent；B5 whole-session thin UI只消费backend blocker/outcome。
  ADR 0012 的 repo-local fault + receipt 原子 settlement保持不变。B6 已为 STORE-021 登记真实 browser
  producer；最终 candidate current-HEAD receipt仍须重跑。
- 响应丢失后，相同请求返回 stored receipt，不重复 append；terminal replay 不依赖当前远端 locator，Pending replay 只从 sealed artifacts + Ledger 幂等收敛 Projection。
- Pending response 必须明确表示 Ledger 已提交且 Projection recovery 未完成，不能伪装成未提交。

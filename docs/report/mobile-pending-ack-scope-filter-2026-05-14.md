# Mobile PendingAck Scope Filter

日期：2026-05-14

## 结论

移动端 footer 的 PendingAck 计数已与 desktop bottom bar 收敛到同一 repo/scope 过滤语义。

## 变更

- `apps/web/src/components/mobile_layout/footer_status.rs` 不再直接按 `doc_id` 统计全部 `pending_local_edits`。
- 移动端 footer 现在通过 `PendingScope::from_repo_id_str` 与 `pending_count_for_doc_in_scope` 只统计当前 `repo_id + scope_nonce + doc_id` 的 pending。
- 新增单元测试覆盖同一文档下不同 repo 与不同 scope 的 pending 不污染当前移动端状态。

## 验证

- `cargo test -p deve_web pending_ack_count_uses_current_repo_scope -- --nocapture`
- `cargo test -p deve_web scoped_read_helpers_ignore_other_repo_or_scope -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p deve_web --all-targets -- -D warnings`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`

## 后续

- Acceptance/release guard cleanup 仍是下一批主线缺口。

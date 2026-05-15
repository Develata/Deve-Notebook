# Mainline Coverage Alignment Batch A - 2026-05-16

本报告记录 `mainline-gap-scan-after-native-target-host-closure-2026-05-16.md` 后的第一批 docs/code 对齐。`docs/plan/` 未修改。

## Scope

- Fix `RENDER-LARGE-001` acceptance overclaim.
- Rebind `WEBWRITE-FEAT-01/02/03` to actual Pending -> Ack, Reject cleanup, and repo-scoped writer readiness.
- Preserve pending navigation coverage under distinct `WEBNAV-FEAT-01/02/03` case ids.

## Changes

- `RENDER-LARGE-001` now asserts snapshot-first, progressive replay, and search gate behavior instead of complete virtual rendering.
- `scripts/check-large-doc-baseline.sh` now fails if `RENDER-LARGE-001` claims `virtual_render_enabled true`.
- `WEBWRITE-FEAT-01` now binds to CLI Ack idempotency and Web pending-overlay clear on confirmed echo.
- `WEBWRITE-FEAT-02` now binds to CLI `EditRejected` and Web pending-overlay clear after reject.
- `WEBWRITE-FEAT-03` now binds to repo/scope writer readiness and stale-scope replay suppression.
- Pending navigation cases moved to `WEBNAV-FEAT-01/02/03` and the feature/operation coverage registry was updated accordingly.

## Verification

Ran:

- `scripts/check-large-doc-baseline.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/plan-coverage.sh`
- `cargo test -p deve_cli duplicate_client_op -- --nocapture`
- `cargo test -p deve_web echoed_new_op_clears_matching_pending_overlay -- --nocapture`
- `cargo test -p deve_web stale_edit_rejected_clears_matching_retained_pending_without_banner -- --nocapture`
- `cargo test -p deve_web writer_ready_requires_matching_repo_and_scope_nonce -- --nocapture`
- `cargo test -p deve_web write_ready_resend_sends_pending_edit_when_native_runtime_is_ready -- --nocapture`
- `cargo test -p deve_web write_ready_resend_skips_pending_edit_from_stale_scope -- --nocapture`
- `cargo test -p deve_web large_doc_search_gate -- --nocapture`

Results:

- Large-doc baseline: pass.
- Acceptance bindings: `101` automated, `60` feature walkthrough, `29` manual, `0` unbound soft cases.
- Feature operation paths: pass.
- Plan coverage: `0` blocking violations, `17` existing soft file-size warnings.
- Targeted CLI/Web tests: pass.

## Decision

Batch A is closed. Next executable work is Batch B: add precise acceptance/baseline bindings for `.notegit/.git` segment ignore, `.notegit-backup` negative case, ledger JSON Lines export, and writeback-failure Ack.

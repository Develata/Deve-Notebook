# Mainline Coverage Alignment Batch B - 2026-05-16

本报告记录 Batch A 后的 storage/server 覆盖绑定。`docs/plan/` 未修改。

## Scope

- Bind `.notegit/` and `.git/` segment-level internal path filtering to storage acceptance.
- Bind `.notegit-backup` sibling path as a negative prefix-regression case.
- Bind ledger JSON Lines export behavior to storage acceptance.
- Bind writeback-failure Ack behavior to storage acceptance.

## Changes

- `STORE-007` now covers internal repo path segment semantics and watcher filtering for `.notegit/` / `.git/`.
- `STORE-007` now covers `.notegit-backup/` as a normal sibling path, not an internal path by prefix accident.
- `STORE-014` binds JSON Lines export to monotonic, one-row-per-line ledger serialization and structure-fact coverage.
- `STORE-015` binds the server edit path where ledger append succeeds but workspace projection writeback fails; Ack remains valid and the writeback fault is reported separately.
- `scripts/check-storage-repo-baseline.sh` now guards these bindings against acceptance drift.

## Verification

Ran:

- `scripts/check-storage-repo-baseline.sh`
- `scripts/check-acceptance-bindings.sh`
- `scripts/check-release-baseline.sh`
- `scripts/plan-coverage.sh`
- `git diff --check`
- `cargo test -p deve_core internal_repo_path_uses_segment_semantics -- --nocapture`
- `cargo test -p deve_core --test watcher_internal_ignore -- --nocapture`
- `cargo test -p deve_cli jsonl_roundtrip_is_monotonic_and_line_stable -- --nocapture`
- `cargo test -p deve_cli includes_dir_structure_fact_in_export -- --nocapture`
- `cargo test -p deve_cli edit_acknowledges_ledger_commit_when_workspace_writeback_fails -- --nocapture`

Results:

- Storage/repo baseline: pass.
- Acceptance bindings: `103` automated, `60` feature walkthrough, `29` manual, `0` unbound soft cases.
- Release baseline: pass.
- Plan coverage: `0` blocking violations, `17` existing soft file-size warnings.
- Targeted core/CLI tests: pass.
- Diff hygiene: pass.

## Decision

Batch B is closed. Next executable work is Modal Focus Contract Closure.
